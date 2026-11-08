<script lang="ts">
  // A área Rede: por onde o PC sai (cabo, Wi-Fi, VPN), a rota salto a salto com o dono de cada trecho, e a varredura
  // de Wi-Fi segurada durante o jogo. O veredito e as frases vêm prontos do Rust: a tela não reinterpreta número.

  import { onMount } from "svelte";
  import {
    ponte,
    type DiagnosticoDoWifi,
    type PainelDaBanda,
    type PainelDaVarredura,
    type ResultadoDaBanda,
    type Rota,
    type SaltoDaRota,
    type TrechoDaRota,
  } from "../lib/ponte";

  const NOME_DO_TRECHO: Record<TrechoDaRota, string> = {
    casa: "Sua casa",
    tunel: "VPN",
    operadora: "Operadora",
    internet: "Internet",
    caminho: "Caminho",
    servidor: "Servidor",
    sem_resposta: "Sem resposta",
  };

  const NOME_DO_SINAL = { Forte: "forte", Bom: "bom", Fraco: "fraco", MuitoFraco: "muito fraco" } as const;

  let wifi = $state<DiagnosticoDoWifi | null>(null);
  let erroWifi = $state<string | null>(null);
  let varredura = $state<PainelDaVarredura | null>(null);
  let erroVarredura = $state<string | null>(null);
  let mudandoVarredura = $state(false);

  type Resposta = { tipo: "ok" | "atencao" | "erro"; linhas: string[] };

  let banda = $state<PainelDaBanda | null>(null);
  let erroBanda = $state<string | null>(null);
  let mudandoBanda = $state(false);
  let respostaBanda = $state<Resposta | null>(null);

  let destino = $state("1.1.1.1");
  let rota = $state<Rota | null>(null);
  let medindo = $state<"destino" | "jogo" | null>(null);
  let erroRota = $state<string | null>(null);

  const saltoDoProblema = $derived(rota?.veredito.tipo === "problema" ? rota.veredito.salto : null);

  onMount(() => {
    ponte.wifi().then((w) => (wifi = w)).catch((e) => (erroWifi = String(e)));
    ponte.varredura().then((v) => (varredura = v)).catch((e) => (erroVarredura = String(e)));
    ponte.banda().then((b) => (banda = b)).catch((e) => (erroBanda = String(e)));
  });

  /** Cada resultado do Rust tem nome; aqui vira a frase da tela, sem prometer o que o Windows não confirmou. */
  function explicar(r: ResultadoDaBanda, agindo: boolean): Resposta {
    switch (r.tipo) {
      case "Aplicado": {
        const n = r.politicas.length;
        const linhas = [
          n > 0 ? `Envio limitado agora: ${n} ${n === 1 ? "programa" : "programas"}.` : "Nenhum programa de fundo enviando agora.",
        ];
        if (r.update_limitado) {
          const c = r.update_confirmado;
          if (c === "Confirmado") linhas.push("Windows Update em segundo plano limitado (o Windows confirmou).");
          else if (c === "NaoSeAplica") linhas.push("Windows Update em segundo plano limitado.");
          else if ("Divergente" in c)
            linhas.push("O limite do Windows Update foi escrito, mas o Windows está usando outro valor (outra política manda mais).");
          else linhas.push("O limite do Windows Update foi escrito, mas não consegui conferir se o Windows está usando.");
        }
        if (r.ressalva) linhas.push(r.ressalva);
        return { tipo: r.ressalva ? "atencao" : "ok", linhas: [...linhas, ...r.avisos] };
      }
      case "NadaALimitar":
        return {
          tipo: "ok",
          linhas: [agindo ? "Nada de fundo para segurar agora." : "Ligado. Age quando um jogo abrir.", ...r.avisos],
        };
      case "SemAdministrador":
        return { tipo: "atencao", linhas: ["Segurar a banda exige abrir o Otimiza como administrador."] };
      case "IndisponivelNesteWindows":
        return { tipo: "erro", linhas: [`Este Windows não deixa: ${r.motivo}`] };
      case "Desligado":
        return { tipo: "ok", linhas: ["Desligado. O que estava limitado voltou."] };
      case "JaAplicado":
        return { tipo: "ok", linhas: ["Já está agindo neste jogo."] };
    }
  }

  async function alternarBanda() {
    if (!banda) return;
    mudandoBanda = true;
    erroBanda = null;
    try {
      const r = await ponte.definirBanda(!banda.ligado);
      banda = await ponte.banda();
      respostaBanda = explicar(r, banda.agindo_em !== null);
    } catch (e) {
      erroBanda = String(e);
    } finally {
      mudandoBanda = false;
    }
  }

  async function abrirComoAdministrador() {
    try {
      const msg = await ponte.abrirComoAdministrador();
      if (msg) erroBanda = msg;
    } catch (e) {
      erroBanda = String(e);
    }
  }

  /** "Otimiza-onedrive.exe" vira "onedrive.exe": o prefixo só marca quem criou a política. */
  function programa(politica: string) {
    return politica.replace(/^otimiza-/i, "");
  }

  async function medir(qual: "destino" | "jogo") {
    medindo = qual;
    erroRota = null;
    try {
      rota = qual === "jogo" ? await ponte.rotaDoJogo() : await ponte.rota(destino);
    } catch (e) {
      erroRota = String(e);
    } finally {
      medindo = null;
    }
  }

  async function alternarVarredura() {
    if (!varredura) return;
    mudandoVarredura = true;
    erroVarredura = null;
    try {
      varredura = await ponte.definirVarredura(!varredura.ligado);
    } catch (e) {
      erroVarredura = String(e);
    } finally {
      mudandoVarredura = false;
    }
  }

  /** `null` é "não medi": travessão, nunca zero. */
  function ms(v: number | null) {
    return v == null ? "—" : `${Math.round(v)} ms`;
  }

  function perda(s: SaltoDaRota) {
    if (s.respondidos === 0 || s.perda_pct == null) return "—";
    return `${Math.round(s.perda_pct)}%`;
  }

  function quem(s: SaltoDaRota) {
    if (!s.endereco) return "não respondeu";
    return s.operadora ? `${s.operadora} · ${s.endereco}` : s.endereco;
  }
</script>

<div class="area-rede">
  <!-- Ligação -->
  <section class="bloco" aria-labelledby="rede-ligacao">
    <h2 id="rede-ligacao">Sua ligação</h2>
    {#if erroWifi}
      <p class="erro">{erroWifi}</p>
    {:else if !wifi}
      <p class="vazio">Lendo a placa de rede…</p>
    {:else}
      {@const l = wifi.ligacao}
      {#if l.tipo === "sem_rede"}
        <p class="destaque">Sem rota para a internet agora.</p>
      {:else if l.tipo === "cabo"}
        <p class="destaque">
          Cabo{l.velocidade_mbps != null ? ` · ${l.velocidade_mbps} Mb/s` : ""}
        </p>
        <p class="detalhe">{l.placa}</p>
      {:else if l.tipo === "virtual"}
        <p class="destaque">VPN ou placa virtual</p>
        <p class="detalhe">{l.placa}</p>
      {:else if l.tipo === "outra"}
        <p class="destaque">{l.placa}</p>
      {:else}
        {@const leitura = l.leitura}
        {#if leitura.estado === "lida" || (leitura.estado === "lista_vazia" && leitura.conexao)}
          {@const c = leitura.conexao!}
          <p class="destaque">Wi-Fi “{c.ssid}” · sinal {NOME_DO_SINAL[c.sinal]} ({c.rssi_dbm} dBm)</p>
          <p class="detalhe">
            {[c.banda, c.canal != null ? `canal ${c.canal}` : null, c.padrao, `enlace ${c.recepcao_mbps} Mb/s`]
              .filter(Boolean)
              .join(" · ")}
          </p>
          {#if leitura.estado === "lida" && leitura.vizinhanca}
            <p class="detalhe">
              {leitura.vizinhanca.mesmo_canal} redes no mesmo canal{leitura.vizinhanca.sobrepostas > 0
                ? `, ${leitura.vizinhanca.sobrepostas} em canal sobreposto`
                : ""} · {leitura.vizinhanca.total_ouvidas} ouvidas no total
            </p>
          {/if}
        {:else if leitura.estado === "sem_permissao_de_localizacao"}
          <p class="destaque">Wi-Fi</p>
          <p class="detalhe">
            O Windows só deixa ler as redes por perto com a localização ligada para apps da área de trabalho
            (Configurações › Privacidade › Localização).
          </p>
        {:else if leitura.estado === "nao_li"}
          <p class="destaque">Wi-Fi</p>
          <p class="detalhe">Não consegui ler o Wi-Fi: {leitura.motivo}</p>
        {:else}
          <p class="destaque">Wi-Fi</p>
          <p class="detalhe">A placa não informou a rede agora.</p>
        {/if}
        <p class="detalhe">{l.placa}</p>
      {/if}

      {#if wifi.achados.length > 0}
        <ul class="achados">
          {#each wifi.achados as a, i (i)}
            <li data-gravidade={a.gravidade}>{a.frase}</li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>

  <!-- Rota -->
  <section class="bloco" aria-labelledby="rede-rota">
    <h2 id="rede-rota">Onde está o problema</h2>
    <form
      class="medir"
      onsubmit={(e) => {
        e.preventDefault();
        void medir("destino");
      }}
    >
      <label class="campo">
        <span>Medir até</span>
        <input bind:value={destino} placeholder="endereço ou nome" disabled={medindo !== null} />
      </label>
      <button class="principal" type="submit" disabled={medindo !== null || destino.trim() === ""}>
        {medindo === "destino" ? "Medindo…" : "Medir rota"}
      </button>
      <button class="secundario" type="button" disabled={medindo !== null} onclick={() => medir("jogo")}>
        {medindo === "jogo" ? "Medindo…" : "Servidor do jogo"}
      </button>
    </form>

    {#if medindo}
      <p class="vazio">Mandando 10 pacotes por salto, espalhados pelo segundo. Leva uns 10 s.</p>
    {:else if erroRota}
      <p class="erro">{erroRota}</p>
    {:else if rota}
      <p class="frase" data-limpa={rota.veredito.tipo === "limpa"}>{rota.frase}</p>
      {#if rota.observacao}<p class="detalhe">{rota.observacao}</p>{/if}

      <ol class="saltos">
        <li class="titulos" aria-hidden="true">
          <span>#</span><span>Trecho</span><span>Quem</span><span>Tempo</span><span>Variação</span><span>Perda</span>
        </li>
        {#each rota.saltos as s (s.numero)}
          <li class:problema={s.numero === saltoDoProblema} data-trecho={s.trecho}>
            <span class="numero">{s.numero}</span>
            <span class="trecho">{NOME_DO_TRECHO[s.trecho]}</span>
            <span class="quem" title={s.endereco ?? undefined}>{quem(s)}</span>
            <span class="numero">{ms(s.mediana_ms)}</span>
            <span class="numero">{ms(s.jitter_ms)}</span>
            <span class="numero">{perda(s)}</span>
          </li>
        {/each}
      </ol>
      {#if rota.atribuicao}<p class="atribuicao">{rota.atribuicao}</p>{/if}
    {/if}
  </section>

  <!-- Banda do segundo plano -->
  <section class="bloco" aria-labelledby="rede-banda">
    <div class="linha">
      <div>
        <h2 id="rede-banda">Segurar a banda do segundo plano durante o jogo</h2>
        <p class="detalhe">{banda?.o_que_faz ?? "Lendo…"}</p>
      </div>
      <button
        class="interruptor"
        role="switch"
        aria-checked={banda?.ligado ?? false}
        aria-labelledby="rede-banda"
        disabled={!banda || mudandoBanda || (!banda.ligado && banda.impedimento !== null)}
        onclick={alternarBanda}
      ><span></span></button>
    </div>
    {#if banda?.impedimento && !banda.ligado}
      <div class="impedimento">
        <span>{banda.impedimento}</span>
        <button class="secundario" onclick={abrirComoAdministrador}>Abrir como administrador</button>
      </div>
    {/if}
    {#if banda?.agindo_em}
      <p class="detalhe agindo">
        Agindo agora em {banda.agindo_em}{banda.politicas.length > 0
          ? `: ${banda.politicas.map(programa).join(", ")}`
          : ""}{banda.update_limitado ? " · Windows Update limitado" : ""}.
      </p>
    {/if}
    {#if respostaBanda}
      <ul class="resposta" data-tipo={respostaBanda.tipo}>
        {#each respostaBanda.linhas as l, i (i)}<li>{l}</li>{/each}
      </ul>
    {/if}
    {#if erroBanda}<p class="erro">{erroBanda}</p>{/if}
  </section>

  <!-- Varredura -->
  <section class="bloco linha" aria-labelledby="rede-varredura">
    <div>
      <h2 id="rede-varredura">Wi-Fi sem varredura durante o jogo</h2>
      <p class="detalhe">
        O Windows procura redes de tempos em tempos, e isso pode dar picos de ping no Wi-Fi. Com isto ligado, a procura
        para enquanto o jogo está aberto e volta quando ele fecha.
      </p>
      {#if varredura?.agindo_em}
        <p class="detalhe agindo">Agindo agora em {varredura.agindo_em}.</p>
      {:else if varredura?.observacao}
        <p class="detalhe">{varredura.observacao}</p>
      {/if}
      {#if erroVarredura}<p class="erro">{erroVarredura}</p>{/if}
    </div>
    <button
      class="interruptor"
      role="switch"
      aria-checked={varredura?.ligado ?? false}
      aria-labelledby="rede-varredura"
      disabled={!varredura || mudandoVarredura}
      onclick={alternarVarredura}
    ><span></span></button>
  </section>
</div>

<style>
  .area-rede {
    display: grid;
    gap: 12px;
    margin-top: 20px;
    max-height: min(62vh, 640px);
    overflow: auto;
    padding-right: 4px;
  }
  .bloco {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
    border-radius: 16px;
    background: var(--vidro);
    border: 1px solid var(--vidro-borda);
  }
  .bloco.linha {
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
  }
  h2 {
    margin: 0 0 2px;
    font-size: 12px;
    font-weight: 500;
    color: var(--tinta-suave);
  }
  .destaque {
    margin: 0;
    font-weight: 600;
  }
  .detalhe,
  .vazio {
    margin: 0;
    font-size: 13px;
    color: var(--tinta-suave);
  }
  .agindo {
    color: var(--sucesso);
    font-weight: 600;
  }
  .erro {
    margin: 0;
    font-size: 13px;
    color: var(--perigo);
  }
  .achados {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .achados li {
    padding: 8px 12px;
    border-radius: 12px;
    font-size: 13px;
    background: var(--vidro-denso);
    border-left: 3px solid var(--ceu-500);
  }
  .achados li[data-gravidade="Atencao"] {
    border-left-color: var(--atencao);
  }
  .achados li[data-gravidade="Bom"] {
    border-left-color: var(--sucesso);
  }
  button,
  input {
    font: inherit;
  }
  button {
    cursor: pointer;
  }
  button:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .medir {
    display: flex;
    align-items: end;
    gap: 8px;
    flex-wrap: wrap;
  }
  .campo {
    flex: 1;
    min-width: 160px;
    display: grid;
    gap: 2px;
    font-size: 12px;
    color: var(--tinta-suave);
  }
  .campo input {
    padding: 7px 12px;
    border-radius: var(--raio-botao);
    border: 1px solid var(--vidro-borda);
    background: var(--vidro-denso);
    color: var(--tinta);
    font-size: 14px;
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
  .frase {
    margin: 8px 0 0;
    font-weight: 600;
    padding-left: 10px;
    border-left: 3px solid var(--atencao);
  }
  .frase[data-limpa="true"] {
    border-left-color: var(--sucesso);
  }
  .saltos {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: grid;
    gap: 2px;
    font-size: 13px;
  }
  .saltos li {
    display: grid;
    grid-template-columns: 22px 84px minmax(0, 1fr) 54px 60px 44px;
    gap: 8px;
    align-items: center;
    padding: 5px 8px;
    border-radius: 8px;
  }
  .saltos .titulos {
    font-size: 11px;
    color: var(--tinta-suave);
  }
  .saltos li[data-trecho="sem_resposta"] {
    color: var(--tinta-suave);
  }
  .saltos li.problema {
    background: rgb(255 230 204 / 0.55);
  }
  :global(:root[data-tema="noite"]) .saltos li.problema {
    background: rgb(255 190 120 / 0.16);
  }
  .saltos .numero {
    text-align: right;
  }
  .saltos li > span:first-child {
    text-align: left;
  }
  .quem {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .atribuicao {
    margin: 4px 0 0;
    font-size: 11px;
    color: var(--tinta-suave);
  }
  .linha {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
  }
  .impedimento {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    font-size: 13px;
    color: var(--atencao);
  }
  .resposta {
    margin: 0;
    padding: 8px 12px 8px 26px;
    border-radius: 12px;
    background: var(--vidro-denso);
    border-left: 3px solid var(--sucesso);
    font-size: 13px;
    display: grid;
    gap: 2px;
  }
  .resposta[data-tipo="atencao"] {
    border-left-color: var(--atencao);
  }
  .resposta[data-tipo="erro"] {
    border-left-color: var(--perigo);
  }
  .interruptor {
    position: relative;
    width: 46px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 13px;
    background: rgb(120 130 160 / 0.45);
    transition: background 160ms ease;
  }
  .interruptor span {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
    transition: transform 160ms ease;
  }
  .interruptor[aria-checked="true"] {
    background: var(--ceu-500);
  }
  .interruptor[aria-checked="true"] span {
    transform: translateX(20px);
  }
  :global(.jogo-aberto) .interruptor,
  :global(.jogo-aberto) .interruptor span {
    transition: none;
  }
</style>
