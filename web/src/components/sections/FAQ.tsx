"use client";

import { AnimatePresence, motion } from "framer-motion";
import { ChevronDown } from "lucide-react";
import { useId, useState } from "react";
import { Reveal } from "@/components/motion/Reveal";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";
import { Section } from "@/components/ui/Section";
import { cn } from "@/lib/cn";

/*
 * Perguntas e respostas do site oficial (`site/src/data/i18n/pt.ts`), resumidas
 * sem mudar o que afirmam. Reembolso e canal de entrega ficaram de fora: lá
 * estão marcados como "confirmar com o dono".
 */
const PERGUNTAS = [
  {
    p: "Isso é mais um otimizador que promete FPS e não entrega nada?",
    r: "É contra isso que o produto foi construído. O Otimiza mede a máquina antes e depois de cada alteração e mostra o número — inclusive quando o número diz que não mudou nada. Os limiares que separam ganho de ruído vieram de um teste que mede a mesma máquina três vezes sem alterar coisa alguma.",
  },
  {
    p: "E se não melhorar nada no meu PC?",
    r: "Ele vai dizer isso, com número. Se a configuração já estiver aplicada, a tela mostra “já otimizado” em vez de fingir trabalho. E, antes de qualquer pagamento, ele mede a sua máquina e mostra o principal problema que encontrou nela.",
  },
  {
    p: "É seguro? Isso pode quebrar meu Windows?",
    r: "Toda alteração grava o estado anterior antes de escrever, e o desfazer restaura idêntico, byte a byte. Ele não desliga as proteções contra Spectre/Meltdown, não mexe no Windows Update, no Defender nem no firewall, não faz “limpeza de registro” e não escreve na BIOS.",
  },
  {
    p: "Preciso de internet para usar?",
    r: "Não. Não há ativação online nem verificação periódica de licença. A única coisa que ele faz pela internet é perguntar ao GitHub se saiu versão nova — e, sem resposta, apenas não avisa.",
  },
  {
    p: "Formatei o Windows. Perdi a minha chave?",
    r: "Não. O código da máquina vem do número de série da placa-mãe, que sobrevive à formatação. Em algumas máquinas o fabricante deixa esse número em branco; nesses casos formatar muda o código, e a reemissão também é gratuita.",
  },
  {
    p: "Por que o Windows diz “editor desconhecido” quando eu instalo?",
    r: "Porque o instalador ainda não tem assinatura digital. Na primeira execução, clique em “Mais informações” e depois em “Executar assim mesmo”. Isso está escrito nas notas de cada versão em vez de escondido.",
  },
];

export function FAQ() {
  const [aberta, setAberta] = useState<number | null>(null);
  const reduce = useReducedMotionSafe();
  const base = useId();

  return (
    <Section id="perguntas" labelledBy="perguntas-titulo">
      <div className="mx-auto max-w-[760px]">
        <Reveal>
          <h2 id="perguntas-titulo" className="font-display text-[26px] font-semibold tracking-[-0.04em] sm:text-[30px]">
            Antes de pagar
          </h2>
        </Reveal>

        <Reveal delay={0.05}>
          <ul className="mt-6">
            {PERGUNTAS.map((item, i) => {
              const aberto = aberta === i;
              const idBotao = `${base}-botao-${i}`;
              const idPainel = `${base}-painel-${i}`;
              return (
                <li key={item.p} className="border-b border-line">
                  <h3>
                    <button
                      type="button"
                      id={idBotao}
                      aria-expanded={aberto}
                      aria-controls={idPainel}
                      onClick={() => setAberta(aberto ? null : i)}
                      className="flex min-h-14 w-full items-center justify-between gap-6 py-5 text-left text-[15px] font-medium tracking-[-0.01em]"
                    >
                      {item.p}
                      <ChevronDown
                        size={17}
                        strokeWidth={1.75}
                        aria-hidden="true"
                        className={cn("shrink-0 text-muted transition-transform duration-200", aberto && "rotate-180 text-fg")}
                      />
                    </button>
                  </h3>
                  <AnimatePresence initial={false}>
                    {aberto && (
                      <motion.div
                        id={idPainel}
                        role="region"
                        aria-labelledby={idBotao}
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: reduce ? 0 : 0.22, ease: [0.22, 1, 0.36, 1] }}
                        className="overflow-hidden"
                      >
                        <p className="max-w-[640px] pb-6 text-[14.5px] leading-[1.65] text-muted">{item.r}</p>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </li>
              );
            })}
          </ul>
        </Reveal>
      </div>
    </Section>
  );
}
