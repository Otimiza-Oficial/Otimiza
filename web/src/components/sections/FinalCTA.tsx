import { Download } from "lucide-react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Reveal } from "@/components/motion/Reveal";
import { ButtonLink } from "@/components/ui/Button";
import { links } from "@/lib/site";

/*
 * Um chão quadriculado visto de cima, desenhado por conta: as transversais se
 * aproximam em progressão geométrica rumo ao horizonte, e as longitudinais
 * convergem para um ponto de fuga acima do bloco. SVG puro, sem 3D do CSS.
 */
const LARGURA = 1200;
const ALTURA = 420;
const FUGA = { x: LARGURA / 2, y: -520 };
const TRANSVERSAIS = Array.from({ length: 9 }, (_, i) => ALTURA - ALTURA * (1 - Math.pow(0.72, i + 1)) * 1.02);
const LONGITUDINAIS = Array.from({ length: 25 }, (_, i) => -1800 + i * 200);

function ChaoEmPerspectiva() {
  return (
    <svg
      aria-hidden="true"
      className="absolute inset-x-0 bottom-0 -z-10 h-[62%] w-full"
      viewBox={`0 0 ${LARGURA} ${ALTURA}`}
      preserveAspectRatio="xMidYMax slice"
    >
      <defs>
        <linearGradient id="chao-some" x1="0" y1="1" x2="0" y2="0">
          <stop offset="0" stopColor="#fff" stopOpacity="1" />
          <stop offset="0.85" stopColor="#fff" stopOpacity="0" />
        </linearGradient>
        <mask id="chao-mascara">
          <rect width={LARGURA} height={ALTURA} fill="url(#chao-some)" />
        </mask>
      </defs>
      <g mask="url(#chao-mascara)" stroke="rgb(255 255 255 / 0.22)" strokeWidth="1">
        {LONGITUDINAIS.map((x) => {
          // Reta do ponto no chão (x, ALTURA) até o ponto de fuga, cortada no topo.
          const t = ALTURA / (ALTURA - FUGA.y);
          return <line key={x} x1={x} y1={ALTURA} x2={x + (FUGA.x - x) * t} y2={0} />;
        })}
        {TRANSVERSAIS.map((y) => (
          <line key={y} x1={0} y1={y} x2={LARGURA} y2={y} />
        ))}
      </g>
    </svg>
  );
}

/**
 * O fechamento: um bloco preto com chão em perspectiva, a logo, uma frase e
 * um botão. Depois de tanta tela, nada aqui compete com a única ação.
 */
export function FinalCTA() {
  return (
    <section aria-labelledby="cta-titulo">
      <div className="frame">
        <div className="frame-inner pb-20 md:pb-24">
          <Reveal>
            <div className="grain relative isolate overflow-hidden rounded-[22px] bg-[#070707] px-6 py-20 text-center text-white shadow-[0_0_0_1px_#000,0_40px_80px_-40px_rgb(0_0_0/0.55)] sm:py-24 lg:py-28">
              <ChaoEmPerspectiva />
              {/* Luz de cima: separa o título do chão sem precisar de gradiente colorido. */}
              <div
                aria-hidden="true"
                className="absolute inset-x-0 top-0 -z-10 h-2/3 bg-[radial-gradient(ellipse_50%_70%_at_50%_0%,rgb(255_255_255/0.12),transparent_70%)]"
              />
              <OtimizaLogo mark={44} className="text-white" />
              <h2
                id="cta-titulo"
                className="font-display mx-auto mt-7 max-w-[760px] text-[36px] leading-[1.05] font-semibold tracking-[-0.05em] text-balance sm:text-[52px] lg:text-[60px]"
              >
                Seu PC, medido de verdade.
              </h2>
              <div className="mt-9 flex justify-center">
                <ButtonLink href={links.baixar} variant="light" size="md">
                  <Download size={15} strokeWidth={2.25} aria-hidden="true" />
                  Baixar para Windows
                </ButtonLink>
              </div>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
