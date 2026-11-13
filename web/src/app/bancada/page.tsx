import type { Metadata } from "next";
import { Footer } from "@/components/layout/Footer";
import { Navbar } from "@/components/layout/Navbar";
import { TabelaDaBancada } from "@/components/bancada/TabelaDaBancada";
import { site } from "@/lib/site";

export const metadata: Metadata = {
  title: "Bancada",
  description: "FPS e 1% pior medidos em partidas de verdade pelos PCs da comunidade do Otimiza, por placa de vídeo e jogo.",
  alternates: { canonical: `${site.url}/bancada/` },
};

export default function BancadaPage() {
  return (
    <>
      <Navbar />
      <main id="conteudo" className="frame">
        <div className="frame-inner py-12 sm:py-16">
          <p className="font-mono text-[11px] tracking-[0.22em] text-subtle uppercase">Bancada pública</p>
          <h1 className="font-display mt-4 text-[clamp(26px,3.4vw,36px)] leading-[1.1] font-semibold tracking-[-0.035em]">
            O que cada placa faz nos jogos, medido de verdade.
          </h1>
          <p className="mt-3 max-w-[62ch] text-[14.5px] leading-[1.6] text-muted">
            Números de quem ligou a Comunidade do Otimiza: anônimos, e só aparecem com 3 PCs ou mais na mesma placa e
            jogo.
          </p>
          <div className="mt-8">
            <TabelaDaBancada />
          </div>
        </div>
      </main>
      <Footer />
    </>
  );
}
