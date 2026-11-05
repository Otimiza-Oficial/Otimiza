import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { cn } from "@/lib/cn";

/*
 * O Cartão de Prova (plano 1.0, 4.4): antes → depois, com o selo de origem.
 *
 * OS NÚMEROS SÃO UMA MEDIÇÃO REAL, não um exemplo: `docs/app/PROGRESS.md`, linha "Tela cheia × janela no FiveM",
 * 09/10/2026. PresentMon, mesmo lugar do mapa, no PC de testes (i3-10100F, GTX 1650, monitor de 180 Hz). Janela sem
 * bordas ("Composed: Copy with GPU GDI") contra tela cheia ("Hardware Composed: Independent Flip"). É a troca que o
 * Jogo liso pede quando vê o jogo apresentando em modo de cópia.
 *
 * Um PC e um jogo. O cartão diz isso na legenda, em vez de deixar o número parecer promessa.
 */
const LINHAS = [
  { metrica: "FPS médio", antes: "61", depois: "85", unidade: "fps" },
  { metrica: "1% pior", antes: "24", depois: "31", unidade: "fps" },
  { metrica: "Quadro mais lento (p99)", antes: "30,2", depois: "21,9", unidade: "ms" },
  { metrica: "Processador por quadro", antes: "16,2", depois: "11,5", unidade: "ms" },
];

export function CartaoDeProva({ className }: { className?: string }) {
  return (
    <figure className={cn("vidro-denso brilho-borda rounded-[24px] p-5 sm:p-6", className)}>
      <div className="flex items-center justify-between gap-3">
        <p className="flex items-center gap-2 text-[13px] font-semibold">
          <OtimizaLogo mark={20} />
          Cartão de Prova
        </p>
        <span className="selo">
          <span aria-hidden="true" className="size-1.5 rounded-full bg-current" />
          Medido
        </span>
      </div>

      <p className="mt-4 text-[12.5px] text-muted">
        FiveM · janela sem bordas <span aria-hidden="true">→</span>
        <span className="sr-only">para</span> tela cheia
      </p>
      <p className="font-display mt-1 text-[30px] leading-[1.05] font-semibold tracking-[-0.045em] sm:text-[34px]">
        +7 fps no 1% pior.
      </p>

      <table className="mt-4 w-full text-[13px]">
        <caption className="sr-only">Antes e depois, medidos com o PresentMon</caption>
        <thead>
          <tr className="text-left text-[11px] text-subtle">
            <th scope="col" className="pb-1.5 font-medium">
              Métrica
            </th>
            <th scope="col" className="pb-1.5 text-right font-medium">
              Antes
            </th>
            <th scope="col" className="pb-1.5 text-right font-medium">
              Depois
            </th>
          </tr>
        </thead>
        <tbody>
          {LINHAS.map((l) => (
            <tr key={l.metrica} className="border-t border-line">
              <th scope="row" className="py-2 pr-2 text-left font-normal text-muted">
                {l.metrica}
              </th>
              <td className="tabular py-2 text-right text-muted">
                {l.antes} <span className="text-[11px]">{l.unidade}</span>
              </td>
              <td className="tabular py-2 pl-3 text-right font-semibold">
                {l.depois} <span className="text-[11px] font-normal text-muted">{l.unidade}</span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <figcaption className="mt-4 border-t border-line pt-3 text-[11.5px] leading-[1.5] text-subtle">
        Medido com o PresentMon no mesmo ponto do mapa, em 09/10/2026, num i3-10100F com GTX 1650. Um PC e um jogo: no
        seu, o Otimiza mede de novo — e diz se não mudou nada.
      </figcaption>
    </figure>
  );
}
