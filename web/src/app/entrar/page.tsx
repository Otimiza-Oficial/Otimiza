import type { Metadata } from "next";
import Image from "next/image";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { SmartLink } from "@/components/ui/SmartLink";
import { site, texturaCetim } from "@/lib/site";
import { LoginForm } from "./LoginForm";

export const metadata: Metadata = {
  title: "Entrar",
  description: "Entre para acompanhar a sua licença do Otimiza.",
  alternates: { canonical: `${site.url}/entrar/` },
  robots: { index: false, follow: true },
};

export default function EntrarPage() {
  return (
    <main id="conteudo" className="grid min-h-dvh bg-[#f3f3f2] lg:grid-cols-[1.08fr_1fr]">
      {/* Lado escuro: cetim, grão e anéis finos. Some no celular, onde só o
          formulário importa. */}
      <section
        aria-hidden="true"
        className="grain relative isolate hidden overflow-hidden bg-[#050505] text-white lg:flex lg:flex-col lg:justify-between lg:p-12 xl:p-14"
      >
        <Image src={texturaCetim} alt="" fill sizes="55vw" className="-z-20 object-cover opacity-35" priority />
        <div className="absolute inset-0 -z-10 bg-[linear-gradient(90deg,#050505_10%,rgb(5_5_5/0.55)_60%,rgb(5_5_5/0.2))]" />
        <svg
          className="absolute -bottom-[38%] left-[18%] -z-10 w-[120%] max-w-none text-white/10"
          viewBox="0 0 800 800"
          fill="none"
        >
          <circle cx="400" cy="400" r="398" stroke="currentColor" />
          <circle cx="400" cy="400" r="300" stroke="currentColor" />
          <circle cx="400" cy="400" r="200" stroke="currentColor" opacity="0.6" />
        </svg>

        <OtimizaLogo mark={34} wordmark className="text-white" />

        <p className="font-display max-w-[520px] text-[52px] leading-[1.02] font-semibold tracking-[-0.055em] xl:text-[64px]">
          A sua licença e o seu PC, em um só lugar.
        </p>

        <p className="text-[13px] text-white/50">Console de desempenho para Windows 10 e 11.</p>
      </section>

      <section className="flex flex-col px-5 py-8 sm:px-8">
        <SmartLink href="/" aria-label="Voltar para a página inicial" className="inline-flex min-h-11 items-center self-start rounded-md lg:hidden">
          <OtimizaLogo mark={28} wordmark />
        </SmartLink>

        <div className="flex flex-1 items-center justify-center py-12">
          <div className="w-full max-w-[380px] rounded-[16px] bg-white p-6 shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_20px_50px_-24px_rgb(0_0_0/0.2)] sm:p-7">
            <h1 className="font-display text-[22px] font-semibold tracking-[-0.04em]">Entrar no Otimiza</h1>
            <p className="mt-1.5 text-[13.5px] leading-[1.5] text-muted">Confira a sua licença, baixe a versão mais nova e fale com o suporte.</p>
            <LoginForm />
          </div>
        </div>

        <SmartLink href="/" className="hidden self-center text-[12.5px] text-muted hover:text-fg lg:inline">
          ← Voltar para o site
        </SmartLink>
      </section>
    </main>
  );
}
