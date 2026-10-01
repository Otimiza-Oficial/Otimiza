import { ScrollVelocity } from "@/components/reactbits/ScrollVelocity";

/*
 * Uma faixa de frases grandes entre o topo e o programa: corre devagar sozinha e acelera com a rolagem
 * (ScrollVelocity, do React Bits). Só o que o produto é, com as palavras do próprio site.
 */
export function Faixa() {
  return (
    <section aria-label="O que o Otimiza faz" className="overflow-hidden border-t border-line py-7 sm:py-9">
      <ScrollVelocity
        faixas={[
          {
            texto: "Medido · Otimizado · Provado com número ·",
            className: "font-display px-2 text-[28px] leading-[1.15] font-semibold tracking-[-0.045em] text-fg sm:text-[40px]",
          },
          {
            texto: "Desfaz byte a byte · Diz quando não há ganho ·",
            className: "font-display texto-contorno px-2 text-[28px] leading-[1.15] font-semibold tracking-[-0.045em] sm:text-[40px]",
          },
        ]}
      />
    </section>
  );
}
