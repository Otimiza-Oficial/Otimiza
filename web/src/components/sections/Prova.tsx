import { Reveal } from "@/components/motion/Reveal";
import { Section, SectionHeading } from "@/components/ui/Section";

/*
 * O método, com as palavras do README ("Como ele é diferente"). Cada passo é uma regra que o programa já segue —
 * nenhum é plano.
 */
const PASSOS = [
  {
    titulo: "Calibra o ruído antes",
    texto:
      "Os limites que separam ganho de acaso vieram de medir a mesma máquina três vezes sem mudar nada. O que variou ali é ruído, e nunca é contado como ganho.",
  },
  {
    titulo: "Mede a travada, não só a média",
    texto:
      "Um congelamento de 40 ms arruína a partida e quase não mexe na média de FPS. Por isso o cartão mostra o 1% pior e o quadro mais lento.",
  },
  {
    titulo: "Compara com e sem, alternando",
    texto:
      "Rodadas com e sem o ajuste, intercaladas, para o aquecimento da máquina não favorecer um lado. O resultado sai com margem de erro.",
  },
  {
    titulo: "Não mede PC ocupado",
    texto:
      "Acima de 25% de uso do processador em segundo plano, nenhum veredito é emitido — e a tela explica por quê.",
  },
];

const SELOS = [
  { nome: "Medido", texto: "Contado pelo programa nesta máquina." },
  { nome: "Estimado", texto: "Calculado, sem medição completa. Dito assim." },
  { nome: "Não medido", texto: "Não deu para medir. Nenhum número inventado no lugar." },
];

export function Prova() {
  return (
    <Section id="prova" labelledBy="prova-titulo">
      <div className="grid gap-10 lg:grid-cols-[1fr_1.15fr] lg:gap-14">
        <div>
          <Reveal>
            <p className="eyebrow">Prova e método</p>
            <SectionHeading
              id="prova-titulo"
              className="mt-3"
              title="Todo número diz de onde veio."
              lead="E admite quando não houve ganho."
            />
            <p className="mt-6 max-w-[460px] text-[15px] leading-[1.65] text-muted">
              Se a configuração já estava certa, a tela mostra “já otimizado” em vez de fingir trabalho. Se a medição
              não separa o depois do antes, o resultado é “dentro do ruído” — e o mascote não comemora.
            </p>
          </Reveal>

          <Reveal delay={0.08}>
            <ul className="mt-8 space-y-2.5">
              {SELOS.map((s) => (
                <li key={s.nome} className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  <span className="selo min-w-[104px] justify-center">{s.nome}</span>
                  <span className="text-[13.5px] text-muted">{s.texto}</span>
                </li>
              ))}
            </ul>
          </Reveal>
        </div>

        <ol className="grid gap-4 sm:grid-cols-2">
          {PASSOS.map((p, i) => (
            <li key={p.titulo} className="flex">
              <Reveal delay={i * 0.05} className="flex w-full">
                <article className="card w-full p-6">
                  <span className="tabular font-mono text-[12px] text-subtle">0{i + 1}</span>
                  <h3 className="font-display mt-4 text-[18px] leading-[1.25] font-semibold tracking-[-0.03em]">
                    {p.titulo}
                  </h3>
                  <p className="mt-2 text-[14px] leading-[1.6] text-muted">{p.texto}</p>
                </article>
              </Reveal>
            </li>
          ))}
        </ol>
      </div>
    </Section>
  );
}
