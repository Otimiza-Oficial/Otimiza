import { Footer } from "@/components/layout/Footer";
import { Navbar } from "@/components/layout/Navbar";
import { FAQ } from "@/components/sections/FAQ";
import { Features } from "@/components/sections/Features";
import { FinalCTA } from "@/components/sections/FinalCTA";
import { Hero } from "@/components/sections/Hero";
import { License } from "@/components/sections/License";
import { Pricing } from "@/components/sections/Pricing";
import { Showcase } from "@/components/sections/Showcase";
import { Support } from "@/components/sections/Support";
import { Turbo } from "@/components/sections/Turbo";

/**
 * A ordem responde as perguntas na ordem em que elas aparecem:
 * o que é → como é → o que há de novo → o que faz → quem me atende → como ativo → quanto custa.
 */
export default function Home() {
  return (
    <>
      <Navbar />
      <main id="conteudo">
        <Hero />
        <Showcase />
        <Turbo />
        <Features />
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
