import { ArrowUpRight, Hash } from "lucide-react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { Ilustrativo } from "@/components/features/FeatureCard";
import { Reveal } from "@/components/motion/Reveal";
import { DecryptedText } from "@/components/reactbits/DecryptedText";
import { ChatAoVivo } from "./ChatAoVivo";
import { ButtonLink } from "@/components/ui/Button";
import { Section, SectionHeading } from "@/components/ui/Section";
import { links, pessoas } from "@/lib/site";

const PONTOS = [
  {
    titulo: "Quem emite a chave é uma pessoa",
    texto: "No momento em que você paga. Não há robô decidindo se você pode usar o programa.",
  },
  {
    titulo: "Formatou? A chave continua valendo",
    texto: "Ela é presa ao número de série da placa-mãe, que sobrevive à formatação.",
  },
  {
    titulo: "Trocou a placa-mãe? Reemissão sem custo",
    texto: "Você manda o código novo e recebe outra chave. Está escrito aqui antes da compra.",
  },
];

export function Support() {
  return (
    <Section id="suporte" labelledBy="suporte-titulo">
      <div className="grid items-center gap-12 lg:grid-cols-[1fr_1.1fr] lg:gap-16">
        <div>
          <Reveal>
            <SectionHeading
              id="suporte-titulo"
              title="Atendimento de gente,"
              lead="direto no Discord."
            />
          </Reveal>

          <ul className="mt-10 space-y-6">
            {PONTOS.map((p, i) => (
              <li key={p.titulo}>
                <Reveal delay={i * 0.05} className="flex gap-4">
                  <DecryptedText text={`0${i + 1}`} className="tabular mt-0.5 font-mono text-[12px] text-subtle" />
                  <div>
                    <h3 className="text-[15.5px] font-semibold tracking-[-0.015em]">{p.titulo}</h3>
                    <p className="mt-1 max-w-[380px] text-[14px] leading-[1.55] text-muted">{p.texto}</p>
                  </div>
                </Reveal>
              </li>
            ))}
          </ul>

          <Reveal delay={0.15}>
            <ButtonLink href={links.discord} variant="secondary" size="md" className="mt-10">
              <BrandLogo brand="discord" size={16} decorative />
              Entrar no Discord
              <ArrowUpRight size={15} strokeWidth={2} aria-hidden="true" />
            </ButtonLink>
          </Reveal>
        </div>

        <Reveal delay={0.08}>
          <ChatMock />
        </Reveal>
      </div>
    </Section>
  );
}

function ChatMock() {
  return (
    <div className="card p-3 sm:p-4" aria-label="Exemplo de conversa no canal de suporte do Discord">
      <div className="sheet overflow-hidden">
        <div className="flex items-center justify-between border-b border-line px-5 py-3.5">
          <p className="flex items-center gap-1.5 text-[13.5px] font-semibold">
            <Hash size={16} strokeWidth={2} className="text-subtle" aria-hidden="true" />
            suporte
          </p>
          <span className="flex items-center gap-1.5 text-[11.5px] text-muted">
            <BrandLogo brand="discord" size={14} decorative />
            Discord
          </span>
        </div>

        <ChatAoVivo
          falas={[
            {
              autor: pessoas.rafael.nome,
              foto: pessoas.rafael.foto,
              hora: "21:04",
              texto: (
                <>
                  Instalei. Apareceu o código da máquina:{" "}
                  <code className="rounded bg-sunken px-1 py-px font-mono text-[12px]">OTZ-7K2M-Q9RD-4F1X</code>
                </>
              ),
            },
            {
              autor: "Otimiza",
              hora: "21:06",
              equipe: true,
              texto: (
                <>
                  Pix confirmado. A chave foi emitida para esse código — cole em{" "}
                  <span className="font-semibold">“A sua chave”</span> e clique em Ativar.
                </>
              ),
            },
            { autor: pessoas.camila.nome, foto: pessoas.camila.foto, hora: "21:11", texto: "Vou formatar o PC no fim de semana. Perco a licença?" },
            { autor: "Otimiza", hora: "21:12", equipe: true, texto: "Não. A chave é presa à placa-mãe — reinstale e cole a mesma." },
            { autor: pessoas.diego.nome, foto: pessoas.diego.foto, hora: "21:15", texto: "Apliquei tudo e ele disse “já otimizado” em metade. Faz sentido?" },
          ]}
        />

        <div className="border-t border-line px-5 py-3">
          <div className="flex h-10 items-center rounded-[10px] bg-sunken px-3.5 text-[13px] text-subtle">
            Conversar em #suporte
          </div>
        </div>
      </div>
      <Ilustrativo className="mt-3" />
    </div>
  );
}
