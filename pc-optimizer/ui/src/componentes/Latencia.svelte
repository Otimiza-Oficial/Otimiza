<script lang="ts">
  // A área Latência: o Caçador de DPC. Mede por alguns segundos qual driver segura o processador (o que o LatencyMon
  // faz) e diz o que fazer com o pior. Só mede: nada é alterado. A captura de kernel exige administrador, e o Rust
  // devolve isso como resultado, não como erro.

  import { onDestroy, onMount } from "svelte";
  import TesteDeReflexo from "./TesteDeReflexo.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri, ponte, type CapturaDpc, type DriverMedido, type TaxaDoMouse } from "../lib/ponte";

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

  const SEGUNDOS_DO_MOUSE = 4;
  // A precisão do ponteiro (aceleração e barra de velocidade fora do meio), de `caminho_do_mouse`.
  type AchadoDoMouse = { titulo: string; explicacao: string; onde_mexer: string };
  let ponteiro = $state<AchadoDoMouse[] | null>(null);
  let mouse = $state<TaxaDoMouse | null>(null);
  let medindoMouse = $state(false);
  let erroMouse = $state<string | null>(null);

  async function medirMouse() {
    medindoMouse = true;
    erroMouse = null;
    try {
      mouse = await ponte.medirMouse(SEGUNDOS_DO_MOUSE);
    } catch (e) {
      erroMouse = String(e);
    } finally {
      medindoMouse = false;
    }
  }

  const relatorio = $derived(resultado?.estado === "Medido" ? resultado.relatorio : null);
  /** Os com problema sempre aparecem; dos saudáveis, só o suficiente para a lista não ficar vazia. */
  const drivers = $derived.by(() => {
    if (!relatorio) return [];
    const ruins = relatorio.drivers.filter((d) => d.nivel !== "Saudavel");
    return ruins.length > 0 ? [...ruins, ...relatorio.drivers.filter((d) => d.nivel === "Saudavel").slice(0, 2)] : relatorio.drivers.slice(0, 5);
  });

  onMount(() => {
    ponte.administrador().then((a) => (admin = a)).catch(() => (admin = null));
    if (noTauri)
      invoke<{ achados: AchadoDoMouse[] }>("caminho_do_mouse", { intervalosUs: [] })
        .then((c) => (ponteiro = c.achados))
        .catch(() => (ponteiro = null));
    else ponteiro = [];
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
  <section class="secao">
    <h2>Qual driver segura o processador</h2>
    <p class="apoio texto">
      Driver que segura o processador por tempo demais vira travadinha, áudio picotando e clique atrasado. Use o PC
      como de costume, ou jogue, enquanto mede. Nada é alterado.
    </p>

    {#if admin === false}
      <div class="recado">
        <span class="ponto" data-tom="atencao"></span>
        <div>
          <span>A medição precisa do Otimiza aberto como administrador: o Windows só entrega esses eventos a ele.</span>
          <button class="btn" onclick={abrirComoAdministrador}>Abrir como administrador</button>
        </div>
      </div>
    {/if}

    <div class="controles">
      <div class="duracoes" role="radiogroup" aria-label="Duração">
        {#each DURACOES as d (d)}
          <button role="radio" aria-checked={segundos === d} disabled={medindo} onclick={() => (segundos = d)}>{d} s</button>
        {/each}
      </div>
      {#if medindo}
        <button class="btn" disabled={parando} onclick={parar}>{parando ? "Parando…" : "Parar agora"}</button>
      {:else}
        <button class="btn primario" disabled={admin === false} onclick={medir}>
          {resultado ? "Medir de novo" : "Começar a medir"}
        </button>
      {/if}
    </div>

    {#if medindo}
      <div class="progresso" role="progressbar" aria-valuemin={0} aria-valuemax={segundos} aria-valuenow={passou}>
        <div style:width={`${(passou / segundos) * 100}%`}></div>
      </div>
      <p class="apoio">Medindo… {passou} de {segundos} s</p>
    {/if}

    {#if erro}
      <div class="recado" role="alert">
        <span class="ponto" data-tom="perigo"></span>
        <div><span>{erro}</span></div>
      </div>
    {/if}

    {#if resultado && resultado.estado !== "Medido"}
      <div class="recado" role="status">
        <span class="ponto" data-tom={resultado.estado === "Falhou" ? "perigo" : "atencao"}></span>
        <div>
          <span>{resultado.texto}</span>
          {#if resultado.estado === "PrecisaDeAdministrador"}
            <button class="btn" onclick={abrirComoAdministrador}>Abrir como administrador</button>
          {/if}
        </div>
      </div>
    {/if}
  </section>

  {#if relatorio}
    <section class="secao">
      <div class="topo">
        <h2>Resultado</h2>
        <span class="selo-medido">Medido</span>
        <span class="apoio">
          {relatorio.duracao_s != null ? `${Math.round(relatorio.duracao_s)} s · ` : ""}{relatorio.eventos.toLocaleString("pt-BR")}
          eventos · {pct(relatorio.pct_total_de_um_nucleo)} de um núcleo
        </span>
      </div>
      <div class="veredito">
        <span
          class="ponto"
          data-tom={relatorio.veredito.tipo === "NenhumPico" ? "ok" : relatorio.veredito.tipo === "DriversComPico" ? "perigo" : "atencao"}
        ></span>
        <div>
          <p class="destaque">{relatorio.veredito.texto}</p>
          {#if relatorio.veredito.tipo === "NenhumPico" && relatorio.veredito.observacao}
            <p class="apoio">{relatorio.veredito.observacao}</p>
          {/if}
          {#each relatorio.avisos as a, i (i)}
            <p class="apoio">{a}</p>
          {/each}
        </div>
      </div>

      {#if drivers.length > 0}
        <table class="drivers">
          <thead>
            <tr>
              <th>Driver</th>
              <th class="n">Pior</th>
              <th class="n">Em 99%</th>
              <th class="n">Acima de 1 ms</th>
            </tr>
          </thead>
          <tbody>
            {#each drivers as d (d.arquivo + d.nome_amigavel)}
              <tr>
                <td>
                  <span class="nome">
                    <span class="ponto" data-tom={d.nivel === "Pico" ? "perigo" : d.nivel === "Atencao" ? "atencao" : "ok"}></span>
                    <span>
                      {nome(d)}{d.atribuicao === "Aproximada" ? " (aprox.)" : ""}
                      <span class="apoio">· {NOME_DO_NIVEL[d.nivel]}</span>
                    </span>
                  </span>
                  {#if d.acao}<p class="acao apoio">{d.acao}</p>{/if}
                </td>
                <td class="n numero">{us(d.maximo_us)}</td>
                <td class="n numero">{d.p99_ate_us == null ? "—" : `até ${us(d.p99_ate_us)}`}</td>
                <td class="n numero">{d.acima_1ms}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>
  {/if}
  <section class="secao">
    <h2>Seu mouse</h2>
    <p class="apoio texto">
      A caixa diz "1000 Hz", mas o programa do fabricante, a porta USB ou o sem-fio podem deixar o mouse em 125. Clique
      em medir e mexa o mouse sem parar, em círculos, por {SEGUNDOS_DO_MOUSE} segundos. Só o instante de cada relato é
      contado: nada de posição nem clique.
    </p>
    <div class="controles">
      <button class="btn primario" disabled={medindoMouse} onclick={medirMouse}>
        {medindoMouse ? "Mexa o mouse…" : mouse ? "Medir de novo" : "Medir o mouse"}
      </button>
    </div>
    {#if ponteiro && ponteiro.length}
      {#each ponteiro as p (p.titulo)}
        <div class="recado">
          <span class="ponto" data-tom="atencao"></span>
          <div><span><b>{p.titulo}.</b> {p.explicacao} {p.onde_mexer}</span></div>
        </div>
      {/each}
    {:else if ponteiro}
      <p class="apoio">Ponteiro sem aceleração e com a barra de velocidade no meio: cada contagem do mouse vira um passo.</p>
    {/if}
    {#if medindoMouse}
      <div class="mexa" aria-hidden="true"><span></span></div>
    {:else if erroMouse}
      <div class="recado" role="alert">
        <span class="ponto" data-tom="perigo"></span>
        <div><span>{erroMouse}</span></div>
      </div>
    {:else if mouse}
      <div class="mouse">
        <strong class="numero">{mouse.nominal ?? (mouse.taxa_hz != null ? Math.round(mouse.taxa_hz) : "—")}<small> Hz</small></strong>
        <div>
          <p>{mouse.frase}</p>
          {#if mouse.estavel_pct != null}
            <p class="apoio">{mouse.relatos} relatos · {Math.round(mouse.estavel_pct)}% no ritmo</p>
          {/if}
        </div>
      </div>
    {/if}
  </section>
  <section class="secao">
    <h2>Teste de reflexo</h2>
    <p class="apoio texto">
      Do verde ao clique entram o seu reflexo e o atraso do PC (mouse, Windows, tela). Faça antes e depois de otimizar,
      descansado, e compare.
    </p>
    <TesteDeReflexo />
  </section>
</div>

<style>
  .texto {
    font-size: var(--t-corpo);
    max-width: 62ch;
  }
  .destaque {
    font-weight: 500;
  }
  .controles {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  /* Controle segmentado: três durações, uma escolhida. */
  .duracoes {
    display: inline-flex;
    padding: 3px;
    border-radius: var(--raio-1);
    background: var(--superficie-2);
    border: 1px solid var(--linha);
  }
  .duracoes button {
    min-width: 56px;
    height: 30px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .duracoes button[aria-checked="true"] {
    background: var(--superficie);
    color: var(--tinta);
    box-shadow: var(--sombra-1);
    font-weight: 500;
  }
  .duracoes button:disabled {
    cursor: default;
  }
  .progresso {
    height: 4px;
    border-radius: 2px;
    background: var(--linha);
    overflow: hidden;
  }
  .progresso div {
    height: 100%;
    background: var(--acao);
    transition: width 900ms linear;
  }
  .mouse {
    display: flex;
    gap: 20px;
    align-items: center;
  }
  .mouse .numero {
    font-size: var(--t-numero);
    white-space: nowrap;
  }
  .mouse small {
    font-size: 14px;
    color: var(--tinta-2);
  }
  .mouse p {
    margin: 0;
  }
  .mexa {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    border: 2px dashed var(--linha-forte);
  }
  .mexa span {
    position: absolute;
    top: -5px;
    left: 50%;
    width: 10px;
    height: 10px;
    margin-left: -5px;
    border-radius: 50%;
    background: var(--acao);
    transform-origin: 5px 33px;
    animation: girar 0.9s linear infinite;
  }
  @keyframes girar {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .mexa span {
      animation: none;
    }
  }
  .topo {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .topo h2 {
    font-size: var(--t-corpo);
    font-weight: 600;
  }
  .veredito {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .veredito .ponto {
    transform: translateY(-1px);
  }
  .veredito > div {
    display: grid;
    gap: 4px;
  }
  .drivers {
    width: 100%;
    border-collapse: collapse;
  }
  .drivers th {
    padding: 0 8px 6px;
    text-align: left;
    font-weight: 500;
    font-size: var(--t-apoio);
    color: var(--tinta-2);
    border-bottom: 1px solid var(--linha);
  }
  .drivers td {
    padding: 10px 8px;
    border-bottom: 1px solid var(--linha);
    vertical-align: top;
  }
  .drivers th:first-child,
  .drivers td:first-child {
    padding-left: 0;
  }
  .drivers tr:last-child td {
    border-bottom: 0;
  }
  .drivers .n {
    text-align: right;
    white-space: nowrap;
  }
  .nome {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .nome .ponto {
    transform: translateY(-1px);
  }
  .acao {
    margin: 4px 0 0 18px;
    font-size: 13px;
    max-width: 52ch;
  }
</style>
