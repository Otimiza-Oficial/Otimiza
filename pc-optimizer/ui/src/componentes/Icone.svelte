<script lang="ts" module>
  // Os ícones da interface, no traço da referência (estilo visionOS): linha de 1,8 num quadro de 24, cantos redondos.
  // `cheio` pinta por dentro, para o item ativo do trilho.
  export const ICONES = {
    casa: "M3.5 10.2 12 3.5l8.5 6.7V19a1.5 1.5 0 0 1-1.5 1.5h-4.5v-6h-5v6H5A1.5 1.5 0 0 1 3.5 19z",
    raio: "M13.5 2.5 5 13.5h6.5l-1 8 8.5-11h-6.5z",
    pilha: "M12 3 2.5 8 12 13l9.5-5zM2.5 12.5 12 17.5l9.5-5M2.5 16.5 12 21.5l9.5-5",
    wifi: "M2.5 9a14 14 0 0 1 19 0M5.5 12.5a9.5 9.5 0 0 1 13 0M8.8 16a5 5 0 0 1 6.4 0M12 19.5h.01",
    cronometro: "M12 8.5v4.5l2.8 1.8M9.5 2.5h5M12 21.5a8.5 8.5 0 1 0 0-17 8.5 8.5 0 0 0 0 17z",
    monitor: "M4 4.5h16a1.5 1.5 0 0 1 1.5 1.5v9a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 15V6A1.5 1.5 0 0 1 4 4.5zM8.5 20.5h7M12 16.5v4",
    relogio: "M3.5 12a8.5 8.5 0 1 0 2.5-6L3.5 8.5M3.5 3.5v5h5M12 7.5v4.5l3 2",
    engrenagem:
      "M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4zM19.4 13.5a7.6 7.6 0 0 0 0-3l2-1.6-2-3.4-2.4 1a7.6 7.6 0 0 0-2.6-1.5L14 2.5h-4l-.4 2.5A7.6 7.6 0 0 0 7 6.5l-2.4-1-2 3.4 2 1.6a7.6 7.6 0 0 0 0 3l-2 1.6 2 3.4 2.4-1a7.6 7.6 0 0 0 2.6 1.5l.4 2.5h4l.4-2.5a7.6 7.6 0 0 0 2.6-1.5l2.4 1 2-3.4z",
    seta_baixo: "M6 9.5l6 6 6-6",
    seta_cima: "M6 14.5l6-6 6 6",
    sino: "M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 2h-15zM10 20.5a2.2 2.2 0 0 0 4 0",
    sol: "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4",
    lua: "M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z",
    busca: "M10.5 17.5a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM15.5 15.5l5 5",
    microfone: "M12 14.5a3 3 0 0 0 3-3v-5a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3zM6.5 11.5a5.5 5.5 0 0 0 11 0M12 17v3.5",
    mais: "M12 5v14M5 12h14",
    controle:
      "M6.5 8.5h11a4 4 0 0 1 4 4v1.5a2.6 2.6 0 0 1-4.7 1.5L15.5 14h-7l-1.3 1.5A2.6 2.6 0 0 1 2.5 14v-1.5a4 4 0 0 1 4-4zM8 10.5v3M6.5 12h3M15.5 11.5h.01M17.5 13h.01",
    grafico: "M4 20V11M10 20V5M16 20v-8M21 20H3",
    chip: "M8 7h8a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H8a1 1 0 0 1-1-1V8a1 1 0 0 1 1-1zM10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4",
    bateria: "M7 6.5h10a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-11a2 2 0 0 1 2-2zM10 3.5h4M12.5 10 10 14h4l-2.5 4",
    desfazer: "M9 14.5 4 9.5l5-5M4 9.5h10.5a5.5 5.5 0 0 1 0 11H11",
    escudo: "M12 2.5 4.5 5.5v6c0 4.5 3.2 8.4 7.5 10 4.3-1.6 7.5-5.5 7.5-10v-6z",
    lista: "M8.5 6.5h12M8.5 12h12M8.5 17.5h12M4 6.5h.01M4 12h.01M4 17.5h.01",
    rota: "M6 19.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM18 9.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM8.5 17H15a3 3 0 0 0 0-6H9a3 3 0 0 1 0-6h6.5",
    vento: "M3 8.5h11a2.5 2.5 0 1 0-2.5-2.5M3 12.5h15a2.5 2.5 0 1 1-2.5 2.5M3 16.5h7",
    play: "M7 4.5v15l12.5-7.5z",
    pausa: "M7.5 4.5h3v15h-3zM13.5 4.5h3v15h-3z",
    voltar: "M18 5.5 9.5 12l8.5 6.5zM6.5 5.5v13",
    avancar: "M6 5.5l8.5 6.5L6 18.5zM17.5 5.5v13",
  } as const;

  export type NomeDoIcone = keyof typeof ICONES;
</script>

<script lang="ts">
  let { nome, tamanho = 20, cheio = false }: { nome: NomeDoIcone; tamanho?: number; cheio?: boolean } = $props();
</script>

<svg class="icone" class:cheio viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true">
  <path d={ICONES[nome]} />
</svg>

<style>
  .icone {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .cheio {
    fill: currentColor;
  }
</style>
