//! 页面栈与路由，以及换页/换缩放时画布与渲染器的重建
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

impl crate::MiniAppWindow {
    pub(crate) fn navigate_to(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        let page_info = self.pages.get(path).ok_or_else(|| format!("Page not found: {}", path))?;
        
        let mut wxml_parser = WxmlParser::new(&page_info.wxml);
        let wxml_nodes = remove_manual_tabbar(&wxml_parser.parse().map_err(|e| format!("WXML error: {}", e))?);

        // 页面 json 声明的自定义组件（`usingComponents`）：模板按标签登记，
        // 样式并入页面样式表（已带作用域前缀），JS 稍后作为模块执行并建实例。
        let mut component_templates = mini_render::parser::template::ComponentTemplates::new();
        let mut component_wxss = String::new();
        for c in &page_info.components {
            match WxmlParser::new(&c.wxml).parse() {
                Ok(nodes) => {
                    component_templates.insert(c.tag.clone(), nodes);
                    component_wxss.push('\n');
                    component_wxss.push_str(&c.wxss);
                }
                Err(e) => eprintln!("⚠️  组件 {} 的 WXML 解析失败: {}", c.tag, e),
            }
        }
        
        // app.wxss 在前、页面 WXSS 在后：同特异性时页面样式因书写顺序更靠后而胜出
        let merged_wxss = format!("{}\n{}\n{}", self.app_wxss, page_info.wxss, component_wxss);
        let mut wxss_parser = WxssParser::new(&merged_wxss);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS error: {}", e))?;
        
        // 上一页的组件实例作废（组件是按页面声明的）
        self.app.eval("__resetPageComponents()").ok();
        // 先登记路由：页面实例的 `this.route` / `getCurrentPages()` 都依赖它，
        // 返回时也靠它判断逻辑层是否已经出过栈
        self.app.eval(&format!("__setPendingRoute({})", serde_json::to_string(path).unwrap_or_else(|_| "''".into()))).ok();
        // 页面 js 同样按模块作用域执行，且每次进入都重跑（`Page({...})` 要重新注册）。
        // path 即页面路由，决定页面里相对 require 的基准目录。
        self.app.load_module_script(path, &page_info.js)?;
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        self.app.eval(&format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json)).ok();
        self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
        // onReady：微信在首次渲染完成后触发；编译端取数据快照时也会走这一步，
        // 窗体不调用会导致两端初始数据不同（例如在 onReady 里补数据的页面）。
        self.app.eval("if(__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
        // 自定义组件：先按「组件路径」执行它的 js（`Component()` 借此注册到该路径），
        // 再建实例。实例必须真的建出来 —— 框架型产物在 attached 里挂载自己的组件树
        // 并触发首次 setData，没有实例的话组件模板里全是空值（底部导航整条空白）。
        app_window::component_mount::mount_page_components(&mut self.app, &page_info.components, &wxml_nodes);
        print_js_output(&self.app);
        
        self.page_stack.push(PageInstance { path: path.to_string(), query, wxml_nodes, stylesheet, component_templates });
        // 事件对象的 timeStamp 以「页面打开」为零点（微信语义）
        self.page_opened_at = Instant::now();
        // 换页必须重取数据快照：新页面的 onLoad 里可能一次 setData 都没有，
        // 那样缓存里还是上一页的数据。
        self.page_data_dirty = true;
        
        let has_tabbar = self.is_tabbar_page(path);
        self.scroll = { let vp = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
                // 初始内容高 = 视口高（max_scroll 为 0）：真实内容高要等第一帧渲染后才知道。
                // 从前这里塞的是写死的 1500，于是内容不足一屏的页面在首帧前也能拉出 800+ 像素空白。
                ScrollController::new(vp, vp) };
        self.needs_redraw = true;
        println!("✅ Page loaded: {}", path);
        Ok(())
    }

    /// 返回上一页（微信 navigateBack 语义）。
    ///
    /// 关键点：**不重新加载上一页的 JS**。页面实例还在逻辑层的页面栈里，重载脚本
    /// 会重新执行 `Page({...})` 并再跑一遍 `onLoad`，用户在上一页的状态（筛选、
    /// 滚动位置、已填表单）全部丢失，而且会重复触发"仅首次进入"的逻辑。
    /// 正确做法是让逻辑层出栈、恢复原实例，只补一次 `onShow`。
    pub(crate) fn navigate_back(&mut self) -> Result<(), String> {
        if self.page_stack.len() <= 1 { return Ok(()); }
        // 离开当前页：先给它 onUnload
        self.app.eval("if(__currentPage && __currentPage.onUnload) __currentPage.onUnload()").ok();
        self.page_stack.pop();
        // 回到的这一页要重新渲染，它离开时留下的那张图用完即弃
        self.back_shots.pop();
        self.interaction.clear_page_state();
        
        if let Some(page) = self.page_stack.last() {
            let path = page.path.clone();
            // 逻辑层出栈并恢复上一页实例（wx.navigateBack 触发时 JS 已自行出栈，
            // 这里用当前路由校验：不一致才补一次出栈，避免重复 pop）
            let current_route = self.app.eval("(__currentPage && __currentPage.route) || ''").unwrap_or_default();
            if current_route.trim_matches('"') != path {
                self.app.eval("__popPage(1)").ok();
            }
            self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
            // 自定义组件的实例要重建：往前走的时候 `navigate_to` 调过
            // `__resetPageComponents()`，把**上一页**的组件实例也一起作废了。
            // 不重建的话返回后组件模板里全是空值 —— tea-app 从二级页返回首页，
            // 底部那条 tab（自定义组件）整条消失，正是这个原因。
            let comps = self.pages.get(&path).map(|i| i.components.clone()).unwrap_or_default();
            if !comps.is_empty() {
                let nodes = self.page_stack.last().map(|p| p.wxml_nodes.clone()).unwrap_or_default();
                self.app.eval("__resetPageComponents()").ok();
                app_window::component_mount::mount_page_components(&mut self.app, &comps, &nodes);
            }
            print_js_output(&self.app);
            
            let has_tabbar = self.is_tabbar_page(&path);
            self.scroll = { let vp = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
                // 初始内容高 = 视口高（max_scroll 为 0）：真实内容高要等第一帧渲染后才知道。
                // 从前这里塞的是写死的 1500，于是内容不足一屏的页面在首帧前也能拉出 800+ 像素空白。
                ScrollController::new(vp, vp) };
            println!("↩️  返回: {}", path);
        }
        // 恢复的是上一页实例，数据快照要重取（返回后可能一次 setData 都没有）
        self.page_data_dirty = true;
        self.fixed_dirty = true;
        self.needs_redraw = true;
        Ok(())
    }

    pub(crate) fn switch_tab(&mut self, path: &str) -> Result<(), String> {
        self.unload_current_page();
        self.page_stack.clear();
        self.back_shots.clear();
        self.interaction.clear_page_state();
        // 切 tab 会销毁原页面栈（微信语义），逻辑层的栈也要一起清，否则越切越长
        self.app.eval("__resetPageStack()").ok();
        self.navigate_to(path.trim_start_matches('/'), HashMap::new())
    }

    /// 销毁页面之前派发 `onUnload`（`switchTab` / `reLaunch` / `redirectTo` 都会销毁页面）。
    ///
    /// 漏掉这一步的后果很具体：页面在 `onUnload` 里 `clearTimeout`/`clearInterval` 的定时器
    /// 会继续跑。tea-app 启动页就是这个写法（`onUnload(){ this.stopTimers() }`），
    /// 它有一个守护定时器兜底跳首页 —— 不派发 `onUnload` 的话，用户已经翻到别的页面了，
    /// 这个定时器还会把他 `switchTab` 拽回首页。`redirectTo` 早就做了这一步，
    /// `switchTab` / `reLaunch` 漏了。
    pub(crate) fn unload_current_page(&mut self) {
        self.app
            .eval("if(__currentPage && __currentPage.onUnload) __currentPage.onUnload()")
            .ok();
    }

    pub(crate) fn setup_canvas(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        let (pw, ph) = ((LOGICAL_WIDTH as f64 * scale_factor) as u32, (CONTENT_HEIGHT as f64 * scale_factor) as u32);
        self.canvas = Some(Canvas::new(pw, ph));
        self.tabbar_canvas = Some(Canvas::new(pw, (tabbar_height() as f64 * scale_factor) as u32));
        self.fixed_canvas = Some(Canvas::new(pw, (LOGICAL_HEIGHT as f64 * scale_factor) as u32));
        // 共享进程内唯一字体实例（加载一次约 0.9s，若每次切页都重载会造成明显卡顿）
        self.text_renderer = mini_render::text::shared_fonts();
    }

    pub(crate) fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            let mut r = WxmlRenderer::new_with_scale(page.stylesheet.clone(), LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32, self.scale_factor as f32);
            r.set_component_templates(page.component_templates.clone());
            self.renderer = Some(r);
            if let Some(ref ct) = self.custom_tabbar {
                self.tabbar_renderer = Some(WxmlRenderer::new_with_scale(ct.stylesheet.clone(), LOGICAL_WIDTH as f32, tabbar_height() as f32, self.scale_factor as f32));
            }
        }
    }

    pub(crate) fn process_navigation(&mut self) {
        if let Some(nav) = self.pending_navigation.take() {
            match nav {
                NavigationRequest::NavigateTo { url } => {
                    let (p, q) = parse_url(&url);
                    // 入栈前给当前页留一张图，侧滑返回时它要出现在新页下面。
                    // 只在这一条分支做：switchTab/reLaunch 会清栈，redirectTo 是替换，
                    // 都没有「上一页」可回。
                    let shot = self.capture_viewport();
                    if self.navigate_to(&p, q).is_ok() {
                        if let Some(s) = shot {
                            self.back_shots.push(s);
                            // 页面栈上限 10（微信语义），快照跟着裁，免得越走越占内存
                            if self.back_shots.len() > 10 { self.back_shots.remove(0); }
                        }
                    }
                }
                NavigationRequest::NavigateBack => { self.navigate_back().ok(); }
                NavigationRequest::SwitchTab { url } => { let (p, _) = parse_url(&url); self.switch_tab(&p).ok(); }
                NavigationRequest::RedirectTo { url } => {
                    // 关闭当前页再开新页：栈深度不变
                    let (p, q) = parse_url(&url);
                    self.app.eval("if(__currentPage && __currentPage.onUnload) __currentPage.onUnload()").ok();
                    self.page_stack.pop();
                    self.app.eval("__popPage(1)").ok();
                    self.interaction.clear_page_state();
                    self.navigate_to(&p, q).ok();
                }
                NavigationRequest::ReLaunch { url } => {
                    let (p, q) = parse_url(&url);
                    self.unload_current_page();
                    self.page_stack.clear();
                    self.back_shots.clear();
                    self.interaction.clear_page_state();
                    self.app.eval("__resetPageStack()").ok();
                    self.navigate_to(&p, q).ok();
                }
            }
            self.update_renderers();
        }
    }
}
