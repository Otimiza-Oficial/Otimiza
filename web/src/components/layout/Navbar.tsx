"use client";

import { AnimatePresence, motion } from "framer-motion";
import { Download, Menu, X } from "lucide-react";
import { useEffect, useState } from "react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";
import { ButtonLink } from "@/components/ui/Button";
import { SmartLink } from "@/components/ui/SmartLink";
import { cn } from "@/lib/cn";
import { links, nav } from "@/lib/site";

/**
 * No topo, a navbar é só uma linha dentro da moldura. Ao rolar, ela vira uma
 * cápsula branca flutuante, com sombra longa e borda de 1px — sai do fluxo
 * visual da página sem virar uma barra pesada de ponta a ponta.
 */
export function Navbar() {
  const [rolou, setRolou] = useState(false);
  const [aberto, setAberto] = useState(false);
  const reduce = useReducedMotionSafe();

  useEffect(() => {
    const aoRolar = () => setRolou(window.scrollY > 12);
    aoRolar();
    window.addEventListener("scroll", aoRolar, { passive: true });
    return () => window.removeEventListener("scroll", aoRolar);
  }, []);

  useEffect(() => {
    if (!aberto) return;
    const aoTeclar = (e: KeyboardEvent) => e.key === "Escape" && setAberto(false);
    document.addEventListener("keydown", aoTeclar);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", aoTeclar);
      document.body.style.overflow = "";
    };
  }, [aberto]);

  const flutuando = rolou || aberto;

  return (
    <header className="sticky top-0 z-50">
      <div className="frame border-transparent">
        <div
          className={cn(
            "transition-[padding] duration-300 ease-[var(--ease-out-soft)]",
            flutuando ? "px-3 pt-3 md:px-6" : "px-0 pt-0",
          )}
        >
          <nav
            aria-label="Principal"
            className={cn(
              "relative flex h-16 items-center justify-between transition-[background-color,box-shadow,border-radius,padding] duration-300 ease-[var(--ease-out-soft)]",
              flutuando
                ? "rounded-[14px] bg-white/85 px-4 shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_10px_30px_-12px_rgb(0_0_0/0.18)] backdrop-blur-md md:px-6"
                : "frame-inner border-b border-line bg-bg",
            )}
          >
            <SmartLink href="/" aria-label="Otimiza — página inicial" className="-my-2 rounded-md py-2">
              <OtimizaLogo mark={26} wordmark />
            </SmartLink>

            <ul className="absolute left-1/2 hidden -translate-x-1/2 items-center gap-8 lg:flex">
              {nav.map((item) => (
                <li key={item.href}>
                  <SmartLink
                    href={item.href}
                    className="rounded-sm text-[13.5px] text-[#3a3a3a] transition-colors hover:text-fg"
                  >
                    {item.label}
                  </SmartLink>
                </li>
              ))}
            </ul>

            <div className="flex items-center gap-2">
              <span className="hidden lg:contents">
                <ButtonLink href={links.entrar} variant="ghost">
                  Entrar
                </ButtonLink>
              </span>
              <span className="hidden min-[430px]:contents">
                <ButtonLink href={links.baixar}>
                  <Download size={14} strokeWidth={2.25} aria-hidden="true" />
                  Baixar grátis
                </ButtonLink>
              </span>
              <button
                type="button"
                className="-mr-2 inline-flex size-11 items-center justify-center rounded-md text-fg lg:hidden"
                aria-expanded={aberto}
                aria-controls="menu-mobile"
                aria-label={aberto ? "Fechar menu" : "Abrir menu"}
                onClick={() => setAberto((v) => !v)}
              >
                {aberto ? <X size={20} strokeWidth={1.75} /> : <Menu size={20} strokeWidth={1.75} />}
              </button>
            </div>
          </nav>
        </div>
      </div>

      <AnimatePresence>
        {aberto && (
          <motion.div
            id="menu-mobile"
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={{ duration: reduce ? 0 : 0.18, ease: [0.22, 1, 0.36, 1] }}
            className="fixed inset-x-0 top-[76px] bottom-0 bg-bg lg:hidden"
          >
            <ul className="frame-inner flex flex-col pt-4">
              {[...nav, { label: "Entrar", href: links.entrar }].map((item) => (
                <li key={item.href} className="border-b border-line">
                  <SmartLink
                    href={item.href}
                    onClick={() => setAberto(false)}
                    className="font-display flex h-14 items-center text-[18px] font-medium tracking-[-0.02em] text-fg"
                  >
                    {item.label}
                  </SmartLink>
                </li>
              ))}
            </ul>
            <div className="frame-inner pt-6">
              <ButtonLink href={links.baixar} size="md" className="w-full" onClick={() => setAberto(false)}>
                <Download size={15} strokeWidth={2.25} aria-hidden="true" />
                Baixar para Windows
              </ButtonLink>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </header>
  );
}
