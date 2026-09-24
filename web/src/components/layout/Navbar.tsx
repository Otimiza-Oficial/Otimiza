"use client";

import { AnimatePresence, motion, useMotionValueEvent, useScroll, useSpring } from "framer-motion";
import { Download, Menu, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";
import { ButtonLink } from "@/components/ui/Button";
import { SmartLink } from "@/components/ui/SmartLink";
import { cn } from "@/lib/cn";
import { links, nav } from "@/lib/site";

/**
 * A navbar: no topo é uma linha; ao rolar vira uma cápsula, por mola (animar padding e raio refaria o layout a cada quadro).
 * A pastilha desliza até a seção na tela (layoutId), uma linha mostra o progresso, e ela sai do caminho ao descer.
 * Com menos movimento pedido: sem mola, sem deslize, sem esconder; o item ativo ganha uma borda.
 */

const ESCONDE_DEPOIS_DE = 90;

export function Navbar() {
  const [rolou, setRolou] = useState(false);
  const [aberto, setAberto] = useState(false);
  const [escondida, setEscondida] = useState(false);
  const [ativo, setAtivo] = useState<string | null>(null);
  const reduce = useReducedMotionSafe();

  const { scrollY, scrollYProgress } = useScroll();
  const progresso = useSpring(scrollYProgress, { stiffness: 140, damping: 26, restDelta: 0.001 });
  const ultimoY = useRef(0);

  useMotionValueEvent(scrollY, "change", (y) => {
    setRolou(y > 12);
    const descendo = y > ultimoY.current;
    if (!aberto) setEscondida(descendo && y > ESCONDE_DEPOIS_DE);
    ultimoY.current = y;
  });

  /*
   * A seção conta como "a que estou lendo" só na faixa do meio da tela, senão duas vizinhas disputam a pastilha.
   */
  useEffect(() => {
    const ids = nav.map((i) => i.href.replace("#", ""));
    const secoes = ids.map((id) => document.getElementById(id)).filter((s): s is HTMLElement => Boolean(s));
    if (!secoes.length) return;

    const observador = new IntersectionObserver(
      (entradas) => {
        const visivel = entradas.filter((e) => e.isIntersecting).sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
        if (visivel) setAtivo(`#${visivel.target.id}`);
        else if (window.scrollY < 80) setAtivo(null);
      },
      { rootMargin: "-45% 0px -45% 0px", threshold: 0 },
    );

    secoes.forEach((s) => observador.observe(s));
    return () => observador.disconnect();
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
  const mola = reduce ? { duration: 0 } : { type: "spring" as const, stiffness: 420, damping: 38, mass: 0.9 };

  return (
    <motion.header
      className="sticky top-0 z-50"
      animate={{ y: escondida && !reduce ? "-100%" : "0%" }}
      transition={reduce ? { duration: 0 } : { type: "spring", stiffness: 320, damping: 34 }}
    >
      <div className="frame border-transparent">
        <motion.div
          animate={{ paddingTop: flutuando ? 12 : 0, paddingLeft: flutuando ? 12 : 0, paddingRight: flutuando ? 12 : 0 }}
          transition={mola}
          className="md:[--lado:24px]"
        >
          <motion.nav
            aria-label="Principal"
            animate={{
              borderRadius: flutuando ? 14 : 0,
              backgroundColor: flutuando ? "rgb(255 255 255 / 0.85)" : "rgb(250 250 250 / 1)",
              boxShadow: flutuando
                ? "0 0 0 1px rgb(10 10 10 / 0.07), 0 10px 30px -12px rgb(0 0 0 / 0.18)"
                : "0 1px 0 0 rgb(10 10 10 / 0.08)",
            }}
            transition={mola}
            className={cn(
              "relative flex h-16 items-center justify-between backdrop-blur-md",
              flutuando ? "px-4 md:px-6" : "frame-inner",
            )}
          >
            <SmartLink href="/" aria-label="Otimiza — página inicial" className="-my-2 rounded-md py-2">
              <OtimizaLogo mark={26} wordmark />
            </SmartLink>

            <ul className="absolute left-1/2 hidden -translate-x-1/2 items-center gap-1 lg:flex">
              {nav.map((item) => {
                const estaAtivo = ativo === item.href;
                return (
                  <li key={item.href} className="relative">
                    <SmartLink
                      href={item.href}
                      aria-current={estaAtivo ? "true" : undefined}
                      className={cn(
                        "relative z-10 block rounded-full px-3.5 py-1.5 text-[13.5px] transition-colors",
                        estaAtivo ? "text-fg" : "text-[#3a3a3a] hover:text-fg",
                      )}
                    >
                      {item.label}
                    </SmartLink>
                    {estaAtivo &&
                      (reduce ? (
                        <span className="absolute inset-x-3 -bottom-0.5 h-px bg-fg" aria-hidden="true" />
                      ) : (
                        <motion.span
                          layoutId="navbar-ativo"
                          className="absolute inset-0 rounded-full bg-[rgb(10_10_10/0.06)]"
                          transition={{ type: "spring", stiffness: 500, damping: 40 }}
                          aria-hidden="true"
                        />
                      ))}
                  </li>
                );
              })}
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

            {/* Quanto da página já passou. Só aparece depois que a cápsula
                aparece, para não riscar o topo da página inteira. */}
            <motion.div
              aria-hidden="true"
              className="pointer-events-none absolute inset-x-3 bottom-0 h-px origin-left bg-fg/25"
              style={{ scaleX: progresso }}
              animate={{ opacity: flutuando ? 1 : 0 }}
              transition={{ duration: reduce ? 0 : 0.2 }}
            />
          </motion.nav>
        </motion.div>
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
              {[...nav, { label: "Entrar", href: links.entrar }].map((item, i) => (
                <motion.li
                  key={item.href}
                  className="border-b border-line"
                  initial={reduce ? false : { opacity: 0, y: -8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: reduce ? 0 : 0.04 + i * 0.035, duration: reduce ? 0 : 0.22, ease: [0.22, 1, 0.36, 1] }}
                >
                  <SmartLink
                    href={item.href}
                    onClick={() => setAberto(false)}
                    className="font-display flex h-14 items-center text-[18px] font-medium tracking-[-0.02em] text-fg"
                  >
                    {item.label}
                  </SmartLink>
                </motion.li>
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
    </motion.header>
  );
}
