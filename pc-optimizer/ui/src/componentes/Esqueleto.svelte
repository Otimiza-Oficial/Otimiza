<script lang="ts">
  // O lugar do conteúdo enquanto ele chega (Plano de refino 2.2.4): barras no formato do que vem, nunca "Lendo…" solto.
  // O brilho para com o jogo aberto (`:root.jogo-aberto`) e com "reduzir movimento"; o rótulo é para o leitor de tela.

  let { linhas = 3, rotulo = "Carregando" }: { linhas?: number; rotulo?: string } = $props();

  // Larguras fixas e variadas: parece texto, e não pula de um carregamento para o outro.
  const LARGURAS = ["92%", "74%", "86%", "58%", "80%", "66%"];
</script>

<div class="esqueleto" role="status" aria-busy="true" aria-live="polite">
  <span class="so-leitor">{rotulo}</span>
  {#each Array.from({ length: Math.max(1, linhas) }, (_, i) => i) as i (i)}
    <span class="barra" class:primeira={i === 0} style:width={LARGURAS[i % LARGURAS.length]} aria-hidden="true"></span>
  {/each}
</div>

<style>
  .esqueleto {
    display: grid;
    gap: 10px;
    padding: 4px 0;
  }
  .barra {
    display: block;
    height: 10px;
    border-radius: 5px;
    background: linear-gradient(90deg, var(--linha) 0%, var(--linha-forte) 50%, var(--linha) 100%);
    background-size: 220% 100%;
    animation: brilho 1.4s ease-in-out infinite;
  }
  .barra.primeira {
    height: 14px;
    border-radius: 7px;
  }
  @keyframes brilho {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -120% 0;
    }
  }
  :global(:root.jogo-aberto) .barra {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .barra {
      animation: none;
    }
  }
  .so-leitor {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
