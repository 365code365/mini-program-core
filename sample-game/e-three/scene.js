// three.js 场景（小游戏与 Chrome 对照页共用同一份，A8.2）：光照立方体 + 环面 + 雾，角度只看帧号（逐帧确定）。
// build(THREE, canvas, w, h, dpr) → { renderer, step(n) }
function build(THREE, canvas, w, h, dpr) {
    var renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: false, preserveDrawingBuffer: true });
    renderer.setPixelRatio(dpr);
    renderer.setSize(w, h, false);
    var scene = new THREE.Scene();
    scene.background = new THREE.Color(0x203040);
    scene.fog = new THREE.Fog(0x203040, 4, 12);
    var cam = new THREE.PerspectiveCamera(50, w / h, 0.1, 100);
    cam.position.set(0, 1.2, 5); cam.lookAt(0, 0, 0);
    scene.add(new THREE.AmbientLight(0xffffff, 0.35));
    var sun = new THREE.DirectionalLight(0xffffff, 1.2); sun.position.set(3, 4, 5); scene.add(sun);
    var cube = new THREE.Mesh(new THREE.BoxGeometry(1.2, 1.2, 1.2), new THREE.MeshStandardMaterial({ color: 0xf0a020, roughness: 0.4, metalness: 0.1 }));
    cube.position.y = 0.8; scene.add(cube);
    var knot = new THREE.Mesh(new THREE.TorusKnotGeometry(0.6, 0.18, 96, 12), new THREE.MeshPhongMaterial({ color: 0x40c0ff, shininess: 60 }));
    knot.position.y = -0.9; scene.add(knot);
    var floor = new THREE.Mesh(new THREE.PlaneGeometry(20, 20), new THREE.MeshLambertMaterial({ color: 0x406040 }));
    floor.rotation.x = -Math.PI / 2; floor.position.y = -2; scene.add(floor);
    return { renderer: renderer, step: function (n) {
        cube.rotation.set(n * 0.02, n * 0.03, 0); knot.rotation.set(0, n * 0.025, n * 0.01);
        renderer.render(scene, cam);
    } };
}
if (typeof module !== 'undefined') module.exports = build;
