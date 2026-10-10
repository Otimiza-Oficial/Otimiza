// O caminho do quadro em 3D: Jogo → Otimiza → CPU e GPU → Frame Boost → Monitor. A cena não mede nada: ela só
// acende o que a tela manda acender. E ela não pode pesar no jogo que está sendo otimizado: sem foco no Otimiza,
// para de desenhar; com foco, no máximo 60 quadros.

import * as T from "three";

export type Alvo = "jogo" | "logo" | "cpu" | "gpu" | "boost" | "monitor";

export interface Cena {
  acender(ligacoes: string[]): void;
  pulso(alvo: Alvo | null): void;
  boost(ligado: boolean): void;
  capa(fonte: HTMLCanvasElement | HTMLImageElement): void;
  redimensionar(): void;
  destruir(): void;
}


function caminhosDoSvg(d: string): T.Shape[] {
  const tokens = d.match(/[MCLZ]|-?\d*\.?\d+/gi) ?? [];
  const caminhos: T.Path[] = [];
  let atual: T.Path | null = null;
  let cmd = "";
  let i = 0;
  const n = () => parseFloat(tokens[i++]);
  while (i < tokens.length) {
    const t = tokens[i];
    if (/^[MCLZ]$/i.test(t)) {
      cmd = t.toUpperCase();
      i++;
      if (cmd === "M") {
        atual = new T.Path();
        caminhos.push(atual);
        atual.moveTo(n(), -n());
        cmd = "L";
      }
      continue;
    }
    if (!atual) {
      i++;
      continue;
    }
    if (cmd === "C") atual.bezierCurveTo(n(), -n(), n(), -n(), n(), -n());
    else if (cmd === "L") atual.lineTo(n(), -n());
    else i++;
  }

  const dentro = (p: T.Vector2, pol: T.Vector2[]) => {
    let c = false;
    for (let a = 0, b = pol.length - 1; a < pol.length; b = a++) {
      if (pol[a].y > p.y !== pol[b].y > p.y && p.x < ((pol[b].x - pol[a].x) * (p.y - pol[a].y)) / (pol[b].y - pol[a].y) + pol[a].x) c = !c;
    }
    return c;
  };
  const pols = caminhos.map((c) => c.getPoints(40));
  const pais = caminhos.map((_, k) => caminhos.findIndex((_, j) => j !== k && dentro(pols[k][0], pols[j])));
  const formas = new Map<number, T.Shape>();
  caminhos.forEach((c, k) => {
    if (pais[k] === -1) {
      const s = new T.Shape();
      s.curves = c.curves;
      s.currentPoint = c.currentPoint;
      formas.set(k, s);
    }
  });
  caminhos.forEach((c, k) => {
    const pai = formas.get(pais[k]);
    if (pais[k] !== -1 && pai) pai.holes.push(c);
  });
  return [...formas.values()];
}

export function criarCena(host: HTMLElement, logoD: string, aoQuadro: (pos: Record<Alvo, { x: number; y: number }>) => void): Cena | null {
  let renderer: T.WebGLRenderer;
  try {
    renderer = new T.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "low-power" });
  } catch {
    return null;
  }
  const reduz = matchMedia("(prefers-reduced-motion: reduce)").matches;
  renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
  renderer.toneMapping = T.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.35;
  host.prepend(renderer.domElement);

  const cena = new T.Scene();
  const camera = new T.PerspectiveCamera(30, 1, 0.1, 200);

  // Estúdio de luz para os reflexos: painéis brancos num quarto preto.
  const pm = new T.PMREMGenerator(renderer);
  const estudio = new T.Scene();
  estudio.add(new T.Mesh(new T.BoxGeometry(40, 40, 40), new T.MeshBasicMaterial({ color: 0x030303, side: T.BackSide })));
  const painelDeLuz = (w: number, h: number, p: [number, number, number], r: [number, number, number], i: number) => {
    const m = new T.Mesh(new T.PlaneGeometry(w, h), new T.MeshBasicMaterial({ color: new T.Color(i, i, i), side: T.DoubleSide }));
    m.position.set(...p);
    m.rotation.set(...r);
    estudio.add(m);
  };
  painelDeLuz(18, 4, [0, 14, 0], [Math.PI / 2, 0, 0], 4);
  painelDeLuz(5, 14, [-15, 5, 3], [0, Math.PI / 2, 0], 2.2);
  painelDeLuz(5, 14, [15, 5, -4], [0, -Math.PI / 2, 0], 1.2);
  painelDeLuz(14, 2.5, [0, 3, -15], [0, 0, 0], 1);
  painelDeLuz(10, 2, [0, 2, 15], [0, Math.PI, 0], 0.6);
  const ambiente = pm.fromScene(estudio, 0.035).texture;
  cena.environment = ambiente;
  const chave = new T.DirectionalLight(0xffffff, 1.5);
  chave.position.set(-4, 10, 6);
  cena.add(chave);

  const M = {
    vidro: new T.MeshPhysicalMaterial({ color: 0x0c0c0d, metalness: 0.15, roughness: 0.16, clearcoat: 1, clearcoatRoughness: 0.06 }),
    metal: new T.MeshStandardMaterial({ color: 0x77777b, metalness: 1, roughness: 0.26 }),
    metalEscuro: new T.MeshStandardMaterial({ color: 0x26262a, metalness: 0.9, roughness: 0.34 }),
    cromo: new T.MeshPhysicalMaterial({ color: 0xffffff, metalness: 1, roughness: 0.07, clearcoat: 1, clearcoatRoughness: 0.04 }),
    placaBase: new T.MeshStandardMaterial({ color: 0x0e0e0f, metalness: 0.35, roughness: 0.55 }),
    faixa: new T.MeshStandardMaterial({ color: 0x111111, emissive: 0xffffff, emissiveIntensity: 0.15 }),
  };

  const retangulo = (w: number, h: number, r: number) => {
    const s = new T.Shape();
    const x = -w / 2;
    const y = -h / 2;
    s.moveTo(x + r, y);
    s.lineTo(x + w - r, y);
    s.quadraticCurveTo(x + w, y, x + w, y + r);
    s.lineTo(x + w, y + h - r);
    s.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    s.lineTo(x + r, y + h);
    s.quadraticCurveTo(x, y + h, x, y + h - r);
    s.lineTo(x, y + r);
    s.quadraticCurveTo(x, y, x + r, y);
    return s;
  };
  const laje = (w: number, h: number, d: number, r: number, mat: T.Material, bev = 0.035) => {
    const g = new T.ExtrudeGeometry(retangulo(w, h, r), { depth: d, bevelEnabled: true, bevelThickness: bev, bevelSize: bev, bevelSegments: 5, curveSegments: 18 });
    g.center();
    return new T.Mesh(g, mat);
  };
  const textura = (fonte: HTMLCanvasElement | HTMLImageElement) => {
    const t = fonte instanceof HTMLCanvasElement ? new T.CanvasTexture(fonte) : new T.Texture(fonte);
    t.colorSpace = T.SRGBColorSpace;
    t.anisotropy = 8;
    t.needsUpdate = true;
    return t;
  };

  const cvBrilho = document.createElement("canvas");
  cvBrilho.width = cvBrilho.height = 128;
  const gB = cvBrilho.getContext("2d")!;
  const rg = gB.createRadialGradient(64, 64, 0, 64, 64, 64);
  rg.addColorStop(0, "rgba(255,255,255,1)");
  rg.addColorStop(0.25, "rgba(255,255,255,.35)");
  rg.addColorStop(1, "rgba(255,255,255,0)");
  gB.fillStyle = rg;
  gB.fillRect(0, 0, 128, 128);
  const texBrilho = new T.CanvasTexture(cvBrilho);
  const halo = (escala: number, opac = 0) => {
    const s = new T.Sprite(new T.SpriteMaterial({ map: texBrilho, color: 0xffffff, transparent: true, opacity: opac, blending: T.AdditiveBlending, depthWrite: false }));
    s.scale.set(escala, escala, 1);
    return s;
  };

  const raiz = new T.Group();
  cena.add(raiz);
  const alvos = {} as Record<Alvo, { obj: T.Object3D; ancora: T.Vector3 }>;

  const chao = new T.Mesh(
    new T.PlaneGeometry(44, 44),
    new T.ShaderMaterial({
      transparent: true,
      depthWrite: false,
      vertexShader: "varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }",
      fragmentShader:
        "varying vec2 vUv; void main(){ vec2 g = fract(vUv*90.0)-0.5; float p = smoothstep(0.09,0.03,length(g)); float f = smoothstep(0.5,0.08,length(vUv-0.5)); gl_FragColor = vec4(vec3(1.0), p*f*0.28); }",
    })
  );
  chao.rotation.x = -Math.PI / 2;
  chao.position.y = -0.02;
  raiz.add(chao);

  // 1. Jogo
  const jogo = new T.Group();
  jogo.position.set(-5.6, 1.55, 0.9);
  jogo.rotation.set(-0.05, 0.5, 0);
  jogo.add(laje(2.2, 3.0, 0.07, 0.2, M.vidro, 0.03));
  const capaMat = new T.MeshBasicMaterial({ color: 0x222222, toneMapped: false });
  const capaMesh = new T.Mesh(new T.PlaneGeometry(2.08, 2.88), capaMat);
  capaMesh.position.z = 0.071;
  jogo.add(capaMesh);
  raiz.add(jogo);
  alvos.jogo = { obj: jogo, ancora: new T.Vector3(0, 1.75, 0) };

  // 2. A marca do Otimiza em cromo
  const geoLogo = new T.ExtrudeGeometry(caminhosDoSvg(logoD), { depth: 70, bevelEnabled: true, bevelThickness: 14, bevelSize: 10, bevelSegments: 6, curveSegments: 28 });
  geoLogo.center();
  geoLogo.scale(0.0031, 0.0031, 0.0031);
  const logo = new T.Mesh(geoLogo, M.cromo);
  const logoGrupo = new T.Group();
  logoGrupo.position.set(-1.9, 1.35, 0);
  logoGrupo.add(logo);
  const haloLogo = halo(4.5, 0.18);
  haloLogo.position.set(0, -1.25, 0);
  logoGrupo.add(haloLogo);
  const anel = new T.Mesh(new T.TorusGeometry(1.35, 0.012, 8, 120), new T.MeshStandardMaterial({ color: 0x111111, emissive: 0xffffff, emissiveIntensity: 0.5 }));
  anel.rotation.x = Math.PI / 2;
  anel.position.y = -1.3;
  logoGrupo.add(anel);
  raiz.add(logoGrupo);
  alvos.logo = { obj: logoGrupo, ancora: new T.Vector3(0, 1.45, 0) };

  // 3. Processador
  const cpu = new T.Group();
  cpu.position.set(1.4, 0.1, -1.9);
  const sub = laje(1.8, 1.8, 0.07, 0.08, M.placaBase, 0.015);
  sub.rotation.x = -Math.PI / 2;
  cpu.add(sub);
  const ihs = laje(1.22, 1.22, 0.1, 0.14, M.metal, 0.03);
  ihs.rotation.x = -Math.PI / 2;
  ihs.position.y = 0.12;
  cpu.add(ihs);
  const cvChip = document.createElement("canvas");
  cvChip.width = cvChip.height = 256;
  const xc = cvChip.getContext("2d")!;
  xc.save();
  xc.translate(78, 40);
  xc.scale(0.155, 0.155);
  xc.fillStyle = "rgba(0,0,0,.4)";
  xc.fill(new Path2D(logoD), "evenodd");
  xc.restore();
  xc.fillStyle = "rgba(0,0,0,.45)";
  xc.font = "500 20px GeistMono, monospace";
  xc.textAlign = "center";
  xc.fillText("OTIMIZA · CPU", 128, 205);
  const gravura = new T.Mesh(new T.PlaneGeometry(1, 1), new T.MeshBasicMaterial({ map: textura(cvChip), transparent: true, toneMapped: false }));
  gravura.rotation.x = -Math.PI / 2;
  gravura.position.y = 0.2;
  cpu.add(gravura);
  for (let k = 0; k < 12; k++) {
    for (const [sx, sz, rot] of [[1, 0, 0], [-1, 0, 0], [0, 1, 1], [0, -1, 1]]) {
      const cap = new T.Mesh(new T.BoxGeometry(0.05, 0.035, 0.1), M.metal);
      const o = -0.66 + k * 0.12;
      cap.position.set(sx ? sx * 0.78 : o, 0.06, sz ? sz * 0.78 : o);
      if (rot) cap.rotation.y = Math.PI / 2;
      cpu.add(cap);
    }
  }
  raiz.add(cpu);
  alvos.cpu = { obj: cpu, ancora: new T.Vector3(0, 0.65, 0) };

  // 4. Placa de vídeo
  const gpu = new T.Group();
  gpu.position.set(1.6, 0.25, 2.1);
  gpu.rotation.y = -0.18;
  const corpo = laje(3.2, 1.36, 0.3, 0.12, M.metalEscuro, 0.03);
  corpo.rotation.x = -Math.PI / 2;
  gpu.add(corpo);
  const tampa = laje(3.08, 1.24, 0.03, 0.1, M.vidro, 0.012);
  tampa.rotation.x = -Math.PI / 2;
  tampa.position.y = 0.2;
  gpu.add(tampa);
  const faixaGpu = new T.Mesh(new T.BoxGeometry(2.6, 0.02, 0.02), M.faixa);
  faixaGpu.position.set(0, 0.05, 0.7);
  gpu.add(faixaGpu);
  const helices: T.Group[] = [];
  for (const fx of [-0.76, 0.76]) {
    const aro = new T.Mesh(new T.TorusGeometry(0.5, 0.03, 14, 64), M.metal);
    aro.rotation.x = Math.PI / 2;
    aro.position.set(fx, 0.23, 0);
    gpu.add(aro);
    const helice = new T.Group();
    helice.position.set(fx, 0.22, 0);
    helice.add(new T.Mesh(new T.CylinderGeometry(0.13, 0.13, 0.06, 32), M.metal));
    for (let k = 0; k < 9; k++) {
      const p = new T.Mesh(new T.BoxGeometry(0.36, 0.01, 0.12), M.metalEscuro);
      const a = (k / 9) * Math.PI * 2;
      p.position.set(Math.cos(a) * 0.27, 0, Math.sin(a) * 0.27);
      p.rotation.set(0.4, -a, 0);
      helice.add(p);
    }
    gpu.add(helice);
    helices.push(helice);
  }
  raiz.add(gpu);
  alvos.gpu = { obj: gpu, ancora: new T.Vector3(0, 0.75, 0) };

  // 5. Frame Boost
  const boost = new T.Group();
  boost.position.set(4.6, 1.3, 0.1);
  boost.rotation.y = -0.62;
  const quadros: T.Mesh<T.ExtrudeGeometry, T.MeshPhysicalMaterial>[] = [];
  for (let k = 0; k < 3; k++) {
    const forma = retangulo(1.7, 1.06, 0.12);
    forma.holes.push(retangulo(1.52, 0.88, 0.07));
    const g = new T.ExtrudeGeometry(forma, { depth: 0.03, bevelEnabled: true, bevelThickness: 0.012, bevelSize: 0.012, bevelSegments: 3, curveSegments: 12 });
    g.center();
    const mat = new T.MeshPhysicalMaterial({ color: 0xffffff, metalness: 0.6, roughness: 0.12, clearcoat: 1, transparent: true, opacity: k === 0 ? 1 : 0.55, emissive: 0xffffff, emissiveIntensity: 0 });
    const q = new T.Mesh(g, mat);
    q.add(new T.Mesh(new T.PlaneGeometry(1.52, 0.88), new T.MeshBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.03, side: T.DoubleSide, depthWrite: false })));
    boost.add(q);
    quadros.push(q);
  }
  const haloBoost = halo(3.2, 0);
  boost.add(haloBoost);
  raiz.add(boost);
  alvos.boost = { obj: boost, ancora: new T.Vector3(0, 0.95, 0) };

  // 6. Monitor, mostrando o jogo
  const mon = new T.Group();
  mon.position.set(6.9, 1.45, 0.3);
  mon.rotation.y = -0.5;
  mon.add(laje(3.1, 1.86, 0.06, 0.07, M.metalEscuro, 0.02));
  const telaMat = new T.MeshBasicMaterial({ color: 0x1a1a1a, toneMapped: false });
  const telaGeo = new T.PlaneGeometry(2.96, 1.72);
  const uv = telaGeo.attributes.uv as T.BufferAttribute;
  for (let k = 0; k < uv.count; k++) uv.setY(k, 0.32 + uv.getY(k) * 0.42);
  const tela = new T.Mesh(telaGeo, telaMat);
  tela.position.z = 0.052;
  mon.add(tela);
  const pescoco = laje(0.22, 1.1, 0.08, 0.04, M.metal, 0.02);
  pescoco.position.set(0, -1.25, -0.12);
  mon.add(pescoco);
  const pe = laje(1.3, 0.7, 0.05, 0.2, M.metal, 0.02);
  pe.rotation.x = -Math.PI / 2;
  pe.position.set(0, -1.8, -0.1);
  mon.add(pe);
  const haloTela = halo(5, 0.05);
  haloTela.position.set(0, 0, -0.3);
  mon.add(haloTela);
  raiz.add(mon);
  alvos.monitor = { obj: mon, ancora: new T.Vector3(0, 1.25, 0) };

  // Conexões
  const P = (x: number, y: number, z: number) => new T.Vector3(x, y, z);
  const pt = { jogo: P(-4.55, 0.05, 1.2), logoE: P(-2.9, 0.05, 0.1), logoD: P(-0.9, 0.05, 0), cpu: P(0.5, 0.05, -1.8), cpuD: P(2.3, 0.05, -1.8), gpu: P(0.1, 0.05, 2.0), gpuD: P(3.2, 0.05, 1.8), boost: P(4.1, 0.05, 0.5), boostD: P(5.3, 0.05, 0.1), mon: P(6.5, 0.05, 0.6) };
  const ligacoes: Record<string, T.Vector3[]> = {
    "jogo-logo": [pt.jogo, P(-3.8, 0.05, 0.9), pt.logoE],
    "logo-cpu": [pt.logoD, P(-0.2, 0.05, -1.4), pt.cpu],
    "logo-gpu": [pt.logoD, P(-0.4, 0.05, 1.6), pt.gpu],
    "cpu-boost": [pt.cpuD, P(3.4, 0.05, -1.1), pt.boost],
    "gpu-boost": [pt.gpuD, P(3.7, 0.05, 1.2), pt.boost],
    "boost-mon": [pt.boostD, P(5.9, 0.05, 0.2), pt.mon],
    "gpu-mon": [pt.gpuD, P(5.0, 0.05, 2.0), pt.mon],
  };
  const fios: Record<string, { curva: T.CatmullRomCurve3; mat: T.MeshStandardMaterial; faiscas: T.Sprite[]; luz: number; alvo: number }> = {};
  for (const [nome, pts] of Object.entries(ligacoes)) {
    const curva = new T.CatmullRomCurve3(pts);
    const mat = new T.MeshStandardMaterial({ color: 0x1c1c1c, metalness: 0.6, roughness: 0.35, emissive: 0xffffff, emissiveIntensity: 0 });
    raiz.add(new T.Mesh(new T.TubeGeometry(curva, 80, 0.022, 10, false), mat));
    const faiscas = Array.from({ length: 5 }, (_, k) => {
      const s = halo(0.42, 0);
      s.userData.t = k / 5;
      raiz.add(s);
      return s;
    });
    fios[nome] = { curva, mat, faiscas, luz: 0, alvo: 0 };
  }

  const estado = { pulsos: {} as Partial<Record<Alvo, number>>, boost: 0, boostAlvo: 0, acesas: 0 };
  const parallax = { x: 0, y: 0, ax: 0, ay: 0 };
  const aoMover = (e: PointerEvent) => {
    const r = host.getBoundingClientRect();
    parallax.ax = (e.clientX - r.left) / r.width - 0.5;
    parallax.ay = (e.clientY - r.top) / r.height - 0.5;
  };
  const aoSair = () => {
    parallax.ax = parallax.ay = 0;
  };
  host.addEventListener("pointermove", aoMover);
  host.addEventListener("pointerleave", aoSair);

  const foco = new T.Vector3(0.9, 0.8, 0.3);
  function redimensionar() {
    const w = host.clientWidth;
    const h = host.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    const fov = (camera.fov * Math.PI) / 180;
    // Meia largura da cena com folga para as etiquetas: em janela estreita nada fica cortado na borda.
    const dist = Math.max(8.9 / (Math.tan(fov / 2) * camera.aspect), 3.9 / Math.tan(fov / 2)) * 1.02;
    camera.position.copy(foco).addScaledVector(new T.Vector3(0, 0.5, 1).normalize(), dist);
    camera.lookAt(foco);
    camera.updateProjectionMatrix();
  }
  const observador = new ResizeObserver(redimensionar);
  observador.observe(host);
  redimensionar();

  const v = new T.Vector3();
  let t0 = performance.now();
  let tempo = 0;
  let ultimoDesenho = 0;
  let quadroId = 0;
  let vivo = true;

  function quadro(agora: number) {
    if (!vivo) return;
    quadroId = requestAnimationFrame(quadro);
    // Sem foco no Otimiza (a pessoa está no jogo), um desenho a cada dois segundos basta para a tela não ficar velha.
    const intervalo = document.hasFocus() ? 1000 / 60 : 2000;
    if (document.hidden || host.offsetParent === null || agora - ultimoDesenho < intervalo) return;
    ultimoDesenho = agora;

    const dt = Math.min(0.05, (agora - t0) / 1000);
    t0 = agora;
    if (!reduz) tempo += dt;
    parallax.x += (parallax.ax - parallax.x) * 0.05;
    parallax.y += (parallax.ay - parallax.y) * 0.05;
    raiz.rotation.y = parallax.x * 0.12;
    raiz.rotation.x = parallax.y * 0.04;
    logo.rotation.y = Math.sin(tempo * 0.6) * 0.45;
    logoGrupo.position.y = 1.35 + Math.sin(tempo * 1.2) * 0.06;
    (anel.material as T.MeshStandardMaterial).emissiveIntensity = 0.35 + Math.sin(tempo * 2) * 0.15;
    jogo.position.y = 1.55 + Math.sin(tempo * 0.9 + 1) * 0.04;
    if (!reduz) helices.forEach((h) => (h.rotation.y += dt * (6 + estado.boost * 10)));
    estado.boost += (estado.boostAlvo - estado.boost) * 0.06;
    quadros.forEach((q, k) => {
      q.position.z = -k * (0.1 + estado.boost * 0.42);
      q.position.x = k * estado.boost * 0.12;
      q.material.emissiveIntensity = estado.boost * (k === 0 ? 0.25 : 0.55);
      q.material.opacity = k === 0 ? 1 : 0.35 + estado.boost * 0.45;
    });
    haloBoost.material.opacity = estado.boost * 0.35;
    if (telaMat.map) telaMat.color.setScalar(0.3 + estado.acesas * 0.12 + estado.boost * 0.45);
    haloTela.material.opacity = 0.04 + estado.boost * 0.22;
    M.faixa.emissiveIntensity = 0.15 + estado.boost * 1.6;
    for (const f of Object.values(fios)) {
      f.luz += (f.alvo - f.luz) * 0.07;
      f.mat.emissiveIntensity = f.luz * 1.4;
      for (const s of f.faiscas) {
        if (!reduz) s.userData.t = (s.userData.t + dt * 0.12 * (1 + estado.boost)) % 1;
        f.curva.getPointAt(s.userData.t, s.position);
        s.position.y += 0.04;
        s.material.opacity = f.luz * 0.9 * Math.sin(s.userData.t * Math.PI);
      }
    }
    for (const [nome, a] of Object.entries(alvos)) a.obj.scale.setScalar(1 + (estado.pulsos[nome as Alvo] ?? 0) * 0.06);
    renderer.render(cena, camera);

    const w = host.clientWidth;
    const h = host.clientHeight;
    const pos = {} as Record<Alvo, { x: number; y: number }>;
    for (const [nome, a] of Object.entries(alvos)) {
      a.obj.updateMatrixWorld();
      v.copy(a.ancora).applyMatrix4(a.obj.matrixWorld).project(camera);
      pos[nome as Alvo] = { x: (v.x * 0.5 + 0.5) * w, y: (-v.y * 0.5 + 0.5) * h };
    }
    aoQuadro(pos);
  }
  quadroId = requestAnimationFrame(quadro);

  return {
    acender(nomes) {
      for (const [n, f] of Object.entries(fios)) f.alvo = nomes.includes(n) ? 1 : 0;
      estado.acesas = nomes.length ? 1 : 0;
    },
    pulso(alvo) {
      estado.pulsos = alvo ? { [alvo]: 1 } : {};
    },
    boost(ligado) {
      estado.boostAlvo = ligado ? 1 : 0;
    },
    capa(fonte) {
      const t = textura(fonte);
      capaMat.map?.dispose();
      capaMat.map = t;
      capaMat.color.set(0xffffff);
      capaMat.needsUpdate = true;
      telaMat.map = t;
      telaMat.needsUpdate = true;
    },
    redimensionar,
    destruir() {
      vivo = false;
      cancelAnimationFrame(quadroId);
      observador.disconnect();
      host.removeEventListener("pointermove", aoMover);
      host.removeEventListener("pointerleave", aoSair);
      ambiente.dispose();
      pm.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    },
  };
}
