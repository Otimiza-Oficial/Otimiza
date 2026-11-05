import { Footer } from "@/components/layout/Footer";
import { Navbar } from "@/components/layout/Navbar";
import { FAQ } from "@/components/sections/FAQ";
import { FinalCTA } from "@/components/sections/FinalCTA";
import { Hero } from "@/components/sections/Hero";
import { License } from "@/components/sections/License";
import { Pricing } from "@/components/sections/Pricing";
import { Prova } from "@/components/sections/Prova";
import { Recursos } from "@/components/sections/Recursos";
import { Recusa } from "@/components/sections/Recusa";
import { Support } from "@/components/sections/Support";

/**
 * A ordem responde as perguntas na ordem em que elas aparecem:
 * funciona? (a prova no topo) → o que faz → o que não faz → como prova → quem me atende → como ativo → quanto custa.
 */
export default function Home() {
  return (
    <>
      <Navbar />
      <main id="conteudo">
        <Hero />
        <Recursos />
        <Recusa />
        <Prova />
        <Support />
        <License />
        <Pricing />
        <FAQ />
        <FinalCTA />
      </main>
      <Footer />
    </>
  );
}
