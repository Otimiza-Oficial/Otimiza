import { ScrollVelocity } from "@/components/reactbits/ScrollVelocity";

/*
 * Uma faixa de frases grandes entre o topo e o programa: corre devagar sozinha e acelera com a rolagem
 * (ScrollVelocity, do React Bits). Só o que o produto é, com as palavras do próprio site.
 */
export function Faixa() {
  return (
    <section aria-label="O que o Otimiza faz" className="border-t border-line overflow-hidden py-10 sm:py-14">
      <ScrollVelocity
        faixas={[
          {
            texto: "Medido · Otimizado · Provado com número ·",
            className: "font-display px-2 text-[44px] leading-[1.1] font-semibold tracking-[-0.05em] text-fg sm:text-[72px]",
          },
          {
            texto: "Desfaz byte a byte · Diz quando não há ganho ·",
            className: "font-display texto-contorno px-2 text-[44px] leading-[1.1] font-semibold tracking-[-0.05em] sm:text-[72px]",
          },
        ]}
      />
    </section>
  );
}
