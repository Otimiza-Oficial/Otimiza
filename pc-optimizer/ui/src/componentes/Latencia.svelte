<script lang="ts">
  // A área Latência: o Caçador de DPC. Mede por alguns segundos qual driver segura o processador (o que o LatencyMon
  // faz) e diz o que fazer com o pior. Só mede: nada é alterado. A captura de kernel exige administrador, e o Rust
  // devolve isso como resultado, não como erro.

  import { onDestroy, onMount } from "svelte";
  import { ponte, type CapturaDpc, type DriverMedido } from "../lib/ponte";

  const DURACOES = [10, 30, 60] as const;
  const NOME_DO_NIVEL = { Saudavel: "Saudável", Atencao: "Atenção", Pico: "Pico" } as const;

  let segundos = $state<number>(30);
  let admin = $state<boolean | null>(null);
  let medindo = $state(false);
  let parando = $state(false);
  let passou = $state(0);
  let resultado = $state<CapturaDpc | null>(null);
  let erro = $state<string | null>(null);
  let relogio: ReturnType<typeof setInterval> | undefined;

  const relatorio = $derived(resultado?.estado === "Medido" ? resultado.relatorio : null);
  /** Os com problema sempre aparecem; dos saudáveis, só o suficiente para a lista não ficar vazia. */
  const drivers = $derived.by(() => {
    if (!relatorio) return [];
    const ruins = relatorio.drivers.filter((d) => d.nivel !== "Saudavel");
    return ruins.length > 0 ? [...ruins, ...relatorio.drivers.filter((d) => d.nivel === "Saudavel").slice(0, 2)] : relatorio.drivers.slice(0, 5);
  });

  onMount(() => {
    ponte.administrador().then((a) => (admin = a)).catch(() => (admin = null));
  });

  onDestroy(() => clearInterval(relogio));

  async function medir() {
    medindo = true;
    parando = false;
    erro = null;
    resultado = null;
    passou = 0;
    clearInterval(relogio);
    relogio = setInterval(() => (passou = Math.min(segundos, passou + 1)), 1000);
    try {
      resultado = await ponte.capturarDpc(segundos);
    } catch (e) {
      erro = String(e);
    } finally {
      clearInterval(relogio);
      medindo = false;
      parando = false;
    }
  }

  async function parar() {
    parando = true;
    await ponte.cancelarDpc().catch(() => {});
  }

  async function abrirComoAdministrador() {
    try {
      const msg = await ponte.abrirComoAdministrador();
      if (msg) erro = msg;
    } catch (e) {
      erro = String(e);
    }
  }

  function us(v: number | null) {
    if (v == null) return "—";
    return v >= 1000 ? `${(v / 1000).toFixed(1).replace(".", ",")} ms` : `${Math.round(v)} µs`;
  }

  function pct(v: number | null) {
    return v == null ? "—" : `${v.toFixed(2).replace(".", ",")}%`;
  }

  function nome(d: DriverMedido) {
    return d.arquivo && d.nome_amigavel !== `Driver ${d.arquivo}` ? `${d.nome_amigavel} · ${d.arquivo}` : d.nome_amigavel;
  }
</script>

<div class="area-latencia">
  <section class="bloco">
    <h2>Caçador de DPC</h2>
    <p class="detalhe">
      Mede qual driver segura o processador por tempo demais, o que vira travadinha, áudio picotando e clique
      atrasado. Use o PC como de costume (ou jogue) enquanto mede. Nada é alterado.
    </p>

    {#if admin === false}
      <div class="aviso" data-tipo="atencao">
        <strong>A medição precisa do Otimiza aberto como administrador.</strong>
        <span>O Windows só entrega esses eventos do sistema a quem é administrador.</span>
        <button class="secundario" onclick={abrirComoAdministrador}>Abrir como administrador</button>
      </div>
    {/if}

    <div class="controles">
      <div class="duracoes" role="radiogroup" aria-label="Duração">
        {#each DURACOES as d (d)}
          <button
            role="radio"
            aria-checked={segundos === d}
            class="chip"
            disabled={medindo}
            onclick={() => (segundos = d)}>{d} s</button
          >
        {/each}
      </div>
      {#if medindo}
        <button class="secundario" disabled={parando} onclick={parar}>
          {parando ? "Parando…" : "Parar agora"}
        </button>
      {:else}
        <button class="principal" disabled={admin === false} onclick={medir}>
          {resultado ? "Medir de novo" : "Começar a medir"}
        </button>
      {/if}
    </div>

    {#if medindo}
      <div class="progresso" role="progressbar" aria-valuemin={0} aria-valuemax={segundos} aria-valuenow={passou}>
        <div style:width={`${(passou / segundos) * 100}%`}></div>
      </div>
      <p class="detalhe">Medindo… {passou} de {segundos} s</p>
    {/if}
  </section>

  {#if erro}
    <div class="aviso" data-tipo="erro" role="alert"><span>{erro}</span></div>
  {/if}

  {#if resultado && resultado.estado !== "Medido"}
    <div class="aviso" data-tipo={resultado.estado === "Falhou" ? "erro" : "atencao"} role="status">
      <span>{resultado.texto}</span>
      {#if resultado.estado === "PrecisaDeAdministrador"}
        <button class="secundario" onclick={abrirComoAdministrador}>Abrir como administrador</button>
      {/if}
    </div>
  {/if}

  {#if relatorio}
    <section class="bloco">
      <div class="topo">
        <h2>Resultado</h2>
        <span class="selo">Medido</span>
        <span class="detalhe">
          {relatorio.duracao_s != null ? `${Math.round(relatorio.duracao_s)} s` : ""} ·
          {relatorio.eventos.toLocaleString("pt-BR")} eventos · {pct(relatorio.pct_total_de_um_nucleo)} de um núcleo
        </span>
      </div>
      <p class="frase" data-tipo={relatorio.veredito.tipo}>{relatorio.veredito.texto}</p>
      {#if relatorio.veredito.tipo === "NenhumPico" && relatorio.veredito.observacao}
        <p class="detalhe">{relatorio.veredito.observacao}</p>
      {/if}
      {#each relatorio.avisos as a, i (i)}
        <p class="detalhe aviso-linha">{a}</p>
      {/each}

      {#if drivers.length > 0}
        <ul class="drivers">
          {#each drivers as d (d.arquivo + d.nome_amigavel)}
            <li data-nivel={d.nivel}>
              <div class="cabeca">
                <strong title={d.atribuicao === "Aproximada" ? "Atribuição aproximada: o módulo de endereço mais próximo" : undefined}>
                  {nome(d)}{d.atribuicao === "Aproximada" ? " (aprox.)" : ""}
                </strong>
                <span class="nivel">{NOME_DO_NIVEL[d.nivel]}</span>
              </div>
              <div class="numeros">
                <span><b>{us(d.maximo_us)}</b> pior</span>
                <span><b>{d.p99_ate_us == null ? "—" : `até ${us(d.p99_ate_us)}`}</b> em 99%</span>
                <span><b>{d.acima_1ms}</b> acima de 1 ms</span>
                <span><b>{pct(d.pct_de_um_nucleo)}</b> de um núcleo</span>
              </div>
              {#if d.acao}<p class="acao">{d.acao}</p>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style>
  .area-latencia {
    display: grid;
    gap: 12px;
    margin-top: 20px;
    max-height: min(62vh, 640px);
    overflow: auto;
    padding-right: 4px;
  }
  .bloco {
    display: grid;
    gap: 8px;
    padding: 14px 16px;
    border-radius: 16px;
    background: var(--vidro);
    border: 1px solid var(--vidro-borda);
  }
  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--tinta-suave);
  }
  .detalhe {
    margin: 0;
    font-size: 13px;
    color: var(--tinta-suave);
  }
  .aviso {
    display: grid;
    gap: 4px;
    justify-items: start;
    padding: 12px 16px 12px 18px;
    border-radius: 16px;
    background: var(--vidro-denso);
    border-left: 4px solid var(--atencao);
    font-size: 14px;
  }
  .aviso[data-tipo="erro"] {
    border-left-color: var(--perigo);
  }
  .aviso span {
    color: var(--tinta-suave);
  }
  button {
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .controles {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .duracoes {
    display: flex;
    gap: 6px;
    flex: 1;
  }
  .chip {
    padding: 6px 14px;
    border-radius: var(--raio-botao);
    border: 1px solid var(--vidro-borda);
    background: var(--vidro);
    color: var(--tinta);
    font-size: 13px;
  }
  .chip[aria-checked="true"] {
    background: var(--ceu-500);
    border-color: transparent;
    color: white;
  }
  .principal {
    padding: 8px 18px;
    border: 0;
    border-radius: var(--raio-botao);
    font-weight: 600;
    color: white;
    background: var(--ceu-700);
  }
  .secundario {
    padding: 7px 14px;
    border-radius: var(--raio-botao);
    border: 1px solid var(--vidro-borda);
    background: var(--vidro);
    color: var(--tinta);
  }
  .progresso {
    height: 6px;
    border-radius: 3px;
    background: rgb(120 130 160 / 0.25);
    overflow: hidden;
  }
  .progresso div {
    height: 100%;
    background: var(--ceu-500);
    transition: width 900ms linear;
  }
  :global(.jogo-aberto) .progresso div {
    transition: none;
  }
  .topo {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .selo {
    padding: 1px 8px;
    border-radius: var(--raio-botao);
    font-size: 11px;
    font-weight: 600;
    color: var(--ambar-700);
    background: rgb(255 230 204 / 0.85);
  }
  .frase {
    margin: 0;
    font-weight: 600;
    padding-left: 10px;
    border-left: 3px solid var(--sucesso);
  }
  .frase[data-tipo="DriversComPico"] {
    border-left-color: var(--perigo);
  }
  .frase[data-tipo="SemEventos"] {
    border-left-color: var(--atencao);
  }
  .aviso-linha {
    color: var(--atencao);
  }
  .drivers {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .drivers li {
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--vidro-denso);
    border-left: 3px solid var(--sucesso);
  }
  .drivers li[data-nivel="Atencao"] {
    border-left-color: var(--atencao);
  }
  .drivers li[data-nivel="Pico"] {
    border-left-color: var(--perigo);
  }
  .cabeca {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cabeca strong {
    flex: 1;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nivel {
    font-size: 11px;
    font-weight: 600;
    color: var(--tinta-suave);
  }
  .numeros {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 12px;
    color: var(--tinta-suave);
  }
  .numeros b {
    color: var(--tinta);
    font-variant-numeric: tabular-nums;
  }
  .acao {
    margin: 0;
    font-size: 13px;
  }
</style>
