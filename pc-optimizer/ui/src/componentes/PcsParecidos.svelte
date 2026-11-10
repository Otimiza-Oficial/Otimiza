<script lang="ts">
  // "Em PCs iguais ao seu": o que mais ganhou neste jogo em PCs com o mesmo processador e placa, pela comunidade do
  // Otimiza (anônima, só com 3 PCs ou mais). Só leitura: aplicar continua sendo pela calibração, medida neste PC.
  import { ponte, type PcsParecidos } from "../lib/ponte";

  let { jogo }: { jogo: string } = $props();

  let dados = $state<PcsParecidos | null>(null);

  $effect(() => {
    const j = jogo;
    dados = null;
    ponte
      .comunidadeParecidos(j)
      .then((d) => {
        if (j === jogo) dados = d;
      })
      // Sem servidor ou sem peças lidas: o cartão não aparece (no destaque do jogo, erro de rede não ajuda).
      .catch(() => {});
  });

  function frase(receita: string) {
    const [tipo, nome] = receita.includes(":") ? receita.split(/:(.*)/s) : ["", receita];
    if (tipo === "energia") return `Plano de energia "${nome}"`;
    if (tipo === "turbo") return `Receita do Turbo "${nome}"`;
    return receita;
  }

  const sinal = (v: number | null | undefined) => (v == null ? "—" : `${v > 0 ? "+" : ""}${v.toFixed(1).replace(".", ",")}%`);
</script>

{#if dados}
  <div class="parecidos">
    <span class="titulo">Em PCs iguais ao seu</span>
    {#if dados.receitas.length === 0}
      <span class="apoio">
        Ainda poucos PCs com {dados.processador} e {dados.placa} mediram este jogo (precisa de {dados.minimo}).
      </span>
    {:else}
      <span class="apoio">{dados.processador} · {dados.placa}</span>
      <ul>
        {#each dados.receitas.slice(0, 3) as r (r.receita)}
          <li>
            <span class="rot">{frase(r.receita)}</span>
            <span class="numero" title="1% pior">{sinal(r.ganho_low_pct)}</span>
            <span class="apoio">no 1% pior · {r.pcs} PCs</span>
          </li>
        {/each}
      </ul>
      <span class="apoio pequeno">Mediana de cada PC. Calibre a energia aqui para medir no seu.</span>
    {/if}
  </div>
{/if}

<style>
  .parecidos {
    display: grid;
    gap: 6px;
    padding: 16px 0 0;
    border-top: 1px solid var(--linha);
  }
  .titulo {
    font-weight: 600;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }
  li {
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-size: 13px;
  }
  .rot {
    flex: 1;
  }
  .numero {
    color: var(--ok);
    font-weight: 600;
  }
  .pequeno {
    font-size: 11px;
  }
</style>
