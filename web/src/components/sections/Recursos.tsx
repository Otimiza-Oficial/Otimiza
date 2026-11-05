import { BatteryCharging, Monitor, RadioTower, Timer, Zap, type LucideIcon } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { Section, SectionHeading } from "@/components/ui/Section";
import { cn } from "@/lib/cn";
import { VERSAO_EXIBIDA } from "@/lib/site";

/*
 * As cinco áreas de desempenho do plano 1.0 (cap. 6 e 7), cada uma com a promessa HONESTA do próprio plano — inclusive
 * o que ela não faz. A etiqueta diz o que já existe no programa e o que é novo nesta versão; nada aqui tem número,
 * porque número sem medição seria exatamente o que o Otimiza critica.
 */
type Estado = "existe" | "melhorado" | "novo";

const AREAS: { icone: LucideIcon; nome: string; estado: Estado; titulo: string; texto: string; naoFaz?: string }[] = [
  {
    icone: Zap,
    nome: "Turbo",
    estado: "melhorado",
    titulo: "Um toque para o jogo aberto.",
    texto:
      "Dez segundos de jogo dizem o que está no limite — placa ou processador — e ele liga só o que ajuda naquele caso. Na primeira vez, prova com rodadas alternadas, com e sem. Piorou, desliga sozinho.",
  },
  {
    icone: RadioTower,
    nome: "Rede",
    estado: "novo",
    titulo: "Onde o ping pula.",
    texto:
      "Ping, variação e perda durante a partida, e onde está o problema: na sua casa, na operadora ou no servidor. Enquanto você joga, segura os downloads em segundo plano.",
    naoFaz: "Não diminui a distância até o servidor. Ninguém diminui.",
  },
  {
    icone: Timer,
    nome: "Latência",
    estado: "novo",
    titulo: "Do clique à imagem.",
    texto:
      "Mede as etapas que dá para medir entre o clique e a tela, e soma só o que foi medido. O resultado vem como “pelo menos X ms”.",
    naoFaz: "Não promete “1 ms”: esse número não se mede num PC comum.",
  },
  {
    icone: BatteryCharging,
    nome: "Energia",
    estado: "melhorado",
    titulo: "Rápido no jogo, fresco no resto.",
    texto:
      "Com o jogo aberto, o processador trabalha no máximo; fechou, volta ao normal. No notebook, o modo de jogo só liga na tomada.",
  },
  {
    icone: Monitor,
    nome: "Tela",
    estado: "novo",
    titulo: "O monitor que você pagou.",
    texto:
      "Muito monitor de 144 Hz roda a 60 sem o dono saber. O Otimiza compara a taxa atual com a que a tela aguenta e corrige com um clique — que também se desfaz.",
  },
];

const ETIQUETA: Record<Estado, string> = {
  existe: "Já no Otimiza",
  melhorado: `Melhorado na ${VERSAO_EXIBIDA}`,
  novo: `Novo na ${VERSAO_EXIBIDA}`,
};

export function Recursos() {
  return (
    <Section id="recursos" labelledBy="recursos-titulo">
      <Reveal>
        <p className="eyebrow">O que ele faz</p>
        <SectionHeading
          id="recursos-titulo"
          className="mt-3"
          title="Cinco áreas, um jeito de trabalhar."
          lead="Lê, ajusta o que ajuda, mede de novo."
        />
      </Reveal>

      <ul className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-6">
        {AREAS.map((a, i) => (
          <li key={a.nome} className={cn("flex", i < 2 ? "lg:col-span-3" : "lg:col-span-2")}>
            <Reveal delay={i * 0.04} className="flex w-full">
              <article className="card flex w-full flex-col p-6">
                <div className="flex items-center justify-between gap-3">
                  <span className="vidro-denso grid size-12 place-items-center rounded-full! text-ceu-700 dark:text-lavanda-400">
                    <a.icone size={21} strokeWidth={2} aria-hidden="true" />
                  </span>
                  <span
                    className={cn(
                      "rounded-full px-2.5 py-1 text-[11px] font-semibold",
                      a.estado === "novo"
                        ? "bg-ink text-white"
                        : "bg-surface text-fg shadow-[0_0_0_1px_var(--color-line-strong)]",
                    )}
                  >
                    {ETIQUETA[a.estado]}
                  </span>
                </div>
                <h3 className="mt-5 text-[13px] font-semibold tracking-[0.02em] text-muted">{a.nome}</h3>
                <p className="font-display mt-1 text-[20px] leading-[1.2] font-semibold tracking-[-0.03em]">{a.titulo}</p>
                <p className="mt-3 text-[14px] leading-[1.6] text-muted">{a.texto}</p>
                {a.naoFaz && (
                  <p className="mt-auto pt-4 text-[13px] leading-[1.5] text-fg">
                    <span className="font-semibold">O que não faz: </span>
                    {a.naoFaz}
                  </p>
                )}
              </article>
            </Reveal>
          </li>
        ))}
      </ul>

      <Reveal delay={0.1}>
        <p className="mt-6 max-w-[640px] text-[13px] leading-[1.6] text-muted">
          “Novo na {VERSAO_EXIBIDA}” quer dizer construído para esta versão e testado no canal beta antes de chegar a
          você. Cada área segue as mesmas regras: mede antes, guarda o valor anterior e desfaz o que piorar.
        </p>
      </Reveal>
    </Section>
  );
}
