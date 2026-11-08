<script lang="ts">
  // A área Ajustes: o FPS por cima do jogo (sobreposição), o que o Otimiza faz sozinho, a licença, a versão e o
  // relatório de suporte. O tema (claro/escuro) mora na barra de cima, onde a referência põe.
  import { onMount } from "svelte";
  import {
    ponte,
    type AvisoDeVersao,
    type CantoDaSobreposicao,
    type EstadoDaLicenca,
    type ModoDaSobreposicao,
    type Preferencias,
    type Sobreposicao,
    type VersaoDoApp,
  } from "../lib/ponte";

  const MODOS: [ModoDaSobreposicao, string][] = [
    ["Desligado", "Desligado"],
    ["Minimo", "Só o FPS"],
    ["Detalhado", "Detalhado"],
  ];
  const CANTOS: [CantoDaSobreposicao, string][] = [
    ["SuperiorEsquerdo", "↖"],
    ["SuperiorDireito", "↗"],
    ["InferiorEsquerdo", "↙"],
    ["InferiorDireito", "↘"],
  ];

  let sobreposicao = $state<Sobreposicao | null>(null);
  let preferencias = $state<Preferencias | null>(null);
  let licenca = $state<EstadoDaLicenca | null>(null);
  let aviso = $state<AvisoDeVersao | null>(null);
  let versao = $state<VersaoDoApp | null>(null);
  let chave = $state("");
  let ativando = $state(false);
  let recado = $state<{ tom: "ok" | "perigo" | "atencao"; texto: string } | null>(null);

  onMount(() => {
    ponte.sobreposicao().then((s) => (sobreposicao = s)).catch(() => {});
    ponte.preferencias().then((p) => (preferencias = p)).catch(() => {});
    ponte.licenca().then((l) => (licenca = l)).catch(() => {});
    ponte.versao().then((v) => (versao = v)).catch(() => {});
    ponte.versaoMaisNova().then((a) => (aviso = a)).catch(() => {});
  });

  async function gravarSobreposicao(modo: ModoDaSobreposicao, canto: CantoDaSobreposicao, comFb: boolean) {
    try {
      sobreposicao = await ponte.gravarSobreposicao(modo, canto, comFb);
    } catch (e) {
      recado = { tom: "perigo", texto: String(e) };
    }
  }

  async function mostrarAgora() {
    try {
      sobreposicao = await ponte.mostrarSobreposicao(false);
    } catch (e) {
      recado = { tom: "atencao", texto: String(e) };
    }
  }

  async function mudar<K extends keyof Preferencias>(k: K, v: Preferencias[K]) {
    if (!preferencias) return;
    try {
      preferencias = await ponte.gravarPreferencias({ ...preferencias, [k]: v });
    } catch (e) {
      recado = { tom: "perigo", texto: String(e) };
    }
  }

  async function ativar(e: SubmitEvent) {
    e.preventDefault();
    ativando = true;
    recado = null;
    try {
      licenca = await ponte.ativarLicenca(chave.trim());
      recado = licenca.ativa ? { tom: "ok", texto: "Licença ativada. Obrigado!" } : { tom: "perigo", texto: licenca.motivo ?? "A chave não foi aceita." };
      if (licenca.ativa) chave = "";
    } catch (x) {
      recado = { tom: "perigo", texto: String(x) };
    } finally {
      ativando = false;
    }
  }

  async function copiarRelatorio() {
    try {
      await navigator.clipboard.writeText(await ponte.relatorioDeSuporte());
      recado = { tom: "ok", texto: "Relatório copiado. Cole na conversa com o suporte." };
    } catch (e) {
      recado = { tom: "perigo", texto: String(e) };
    }
  }
</script>

<div class="ajustes">
  {#if recado}
    <div class="recado" role="status"><span class="ponto" data-tom={recado.tom}></span><div><span>{recado.texto}</span></div></div>
  {/if}

  <!-- Sobreposição -->
  <section class="secao">
    <div class="linha">
      <div class="info">
        <h2>FPS por cima do jogo</h2>
        <span class="apoio">
          {sobreposicao?.ligado ? `Na tela agora${sobreposicao.jogo ? ` em ${sobreposicao.jogo}` : ""}.` : "Aparece sozinho quando um jogo abre, se o anticheat dele deixar."}
        </span>
      </div>
      {#if sobreposicao && sobreposicao.config.modo !== "Desligado" && !sobreposicao.ligado}
        <button class="btn" onclick={mostrarAgora}>Mostrar agora</button>
      {/if}
    </div>
    {#if sobreposicao}
      {@const c = sobreposicao.config}
      <div class="controles">
        <div class="segmento" role="radiogroup" aria-label="Modo">
          {#each MODOS as [m, nome] (m)}
            <button role="radio" aria-checked={c.modo === m} onclick={() => gravarSobreposicao(m, c.canto, c.com_frame_boost)}>{nome}</button>
          {/each}
        </div>
        <div class="cantos" role="radiogroup" aria-label="Canto da tela">
          {#each CANTOS as [k, seta] (k)}
            <button role="radio" aria-checked={c.canto === k} aria-label={k} disabled={c.modo === "Desligado"} onclick={() => gravarSobreposicao(c.modo, k, c.com_frame_boost)}>{seta}</button>
          {/each}
        </div>
      </div>
      <label class="marcar">
        <input type="checkbox" checked={c.com_frame_boost} onchange={(e) => gravarSobreposicao(c.modo, c.canto, e.currentTarget.checked)} />
        Mostrar junto com o Frame Boost
      </label>
    {/if}
  </section>

  <!-- Comportamento -->
  {#if preferencias}
    <section class="secao">
      <h2>O que o Otimiza faz sozinho</h2>
      <ul class="lista">
        <li>
          <div class="info"><span>Medir o FPS dos jogos sozinho</span><span class="apoio">Só escuta o Windows; não muda nada. É o que enche o Último jogo e a área Jogos.</span></div>
          <button class="interruptor" role="switch" aria-checked={preferencias.medir_quadros_sozinho} aria-label="Medir o FPS dos jogos sozinho" onclick={() => mudar("medir_quadros_sozinho", !preferencias!.medir_quadros_sozinho)}></button>
        </li>
        <li>
          <div class="info"><span>Modo jogo automático</span><span class="apoio">Muda o sistema quando um jogo abre. Vem desligado: nada muda sem você pedir.</span></div>
          <button class="interruptor" role="switch" aria-checked={preferencias.auto_game_mode} aria-label="Modo jogo automático" onclick={() => mudar("auto_game_mode", !preferencias!.auto_game_mode)}></button>
        </li>
        <li>
          <div class="info"><span>Ponto de restauração antes de otimizar</span><span class="apoio">Uma rede de segurança do próprio Windows antes de cada lote.</span></div>
          <button class="interruptor" role="switch" aria-checked={preferencias.restore_point_before_batch} aria-label="Ponto de restauração antes de otimizar" onclick={() => mudar("restore_point_before_batch", !preferencias!.restore_point_before_batch)}></button>
        </li>
        <li>
          <div class="info"><span>Mostrar ajustes que não se aplicam a este PC</span><span class="apoio">Na área Sistema, para saber o que existe.</span></div>
          <button class="interruptor" role="switch" aria-checked={preferencias.show_unavailable} aria-label="Mostrar ajustes que não se aplicam" onclick={() => mudar("show_unavailable", !preferencias!.show_unavailable)}></button>
        </li>
      </ul>
    </section>
  {/if}

  <!-- Licença -->
  <section class="secao">
    <h2>Licença</h2>
    {#if !licenca}
      <p class="apoio">Lendo…</p>
    {:else if licenca.ativa}
      <ul class="lista">
        <li><span class="ponto" data-tom="ok"></span><span class="info">{licenca.origem}</span><span class="apoio">{licenca.expira ? `até ${licenca.expira}` : "sem vencimento"}</span></li>
        {#if licenca.comprador}<li><span class="ponto"></span><span class="info">Em nome de</span><span>{licenca.comprador}</span></li>{/if}
        <li>
          <span class="ponto"></span><span class="info">Esta máquina</span><span class="numero">{licenca.maquina}</span>
        </li>
        <li>
          <span class="ponto" data-tom={licenca.sobrevive_formatacao ? "ok" : "atencao"}></span>
          <span class="info">{licenca.sobrevive_formatacao ? "Continua valendo se você formatar o PC" : "Formatar o PC pede ativar de novo"}</span>
        </li>
      </ul>
    {:else}
      <p class="apoio">{licenca.motivo ?? "Sem licença: medir, diagnosticar e desfazer continuam livres."}</p>
      <form class="ativar" onsubmit={ativar}>
        <input class="campo" bind:value={chave} placeholder="OTZ-XXXX-XXXX-XXXX" aria-label="Chave da licença" />
        <button class="btn primario" disabled={ativando || chave.trim().length < 8}>{ativando ? "Ativando…" : "Ativar"}</button>
      </form>
    {/if}
  </section>

  <!-- Versão e suporte -->
  <section class="secao">
    <h2>Versão e suporte</h2>
    <ul class="lista">
      <li>
        <span class="ponto" data-tom={aviso?.comparacao === "HaVersaoNova" ? "atencao" : "ok"}></span>
        <span class="info">
          Otimiza {versao?.exibida ?? ""}
          <span class="apoio">
            {aviso?.comparacao === "HaVersaoNova"
              ? `· a ${aviso.versao_publicada_exibida ?? aviso.versao_publicada} já saiu`
              : aviso?.comparacao === "NaoSei"
                ? "· não consegui conferir se há versão nova"
                : "· é a mais nova"}
          </span>
        </span>
        {#if aviso?.comparacao === "HaVersaoNova" && aviso.pagina}
          <a class="btn" href={aviso.pagina} target="_blank" rel="noreferrer">Ver a versão nova</a>
        {/if}
      </li>
      <li>
        <span class="ponto"></span>
        <span class="info">Relatório para o suporte<span class="apoio"> · o que o atendimento precisa, sem dados pessoais</span></span>
        <button class="btn" onclick={copiarRelatorio}>Copiar</button>
      </li>
    </ul>
  </section>
</div>

<style>
  .ajustes {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }
  .linha {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .info {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .controles {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
  }
  .segmento,
  .cantos {
    display: flex;
    padding: 3px;
    border-radius: 18px;
    background: var(--superficie-2);
  }
  .segmento button,
  .cantos button {
    height: 30px;
    padding: 0 12px;
    border: 0;
    border-radius: 15px;
    background: transparent;
    color: var(--tinta-2);
    font-size: 13px;
    cursor: pointer;
  }
  .cantos button {
    width: 34px;
    padding: 0;
    font-size: 15px;
  }
  .segmento button[aria-checked="true"],
  .cantos button[aria-checked="true"] {
    background: var(--tinta);
    color: #fff;
  }
  :global(:root[data-tema="noite"]) .segmento button[aria-checked="true"],
  :global(:root[data-tema="noite"]) .cantos button[aria-checked="true"] {
    color: #1d1d1f;
  }
  .cantos button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .marcar {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .lista > li > .interruptor,
  .lista > li > .btn {
    flex: none;
  }
  .ativar {
    display: flex;
    gap: 8px;
  }
  .ativar .campo {
    flex: 1;
    max-width: 320px;
    font-family: "Cascadia Mono", Consolas, monospace;
    letter-spacing: 0.04em;
  }
  a.btn {
    text-decoration: none;
  }
</style>
