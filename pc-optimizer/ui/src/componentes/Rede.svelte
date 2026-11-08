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
  <section class="secao" aria-labelledby="rede-ligacao">
    <h2 id="rede-ligacao">Sua ligação</h2>
    {#if erroWifi}
      <p class="erro">{erroWifi}</p>
    {:else if !wifi}
      <p class="apoio">Lendo a placa de rede…</p>
    {:else}
      {@const l = wifi.ligacao}
      <div class="ligacao">
        {#if l.tipo === "sem_rede"}
          <p class="destaque">Sem rota para a internet agora.</p>
        {:else if l.tipo === "cabo"}
          <p class="destaque">Cabo{l.velocidade_mbps != null ? ` · ${l.velocidade_mbps} Mb/s` : ""}</p>
          <p class="apoio">{l.placa}</p>
        {:else if l.tipo === "virtual"}
          <p class="destaque">VPN ou placa virtual</p>
          <p class="apoio">{l.placa}</p>
        {:else if l.tipo === "outra"}
          <p class="destaque">{l.placa}</p>
        {:else}
          {@const leitura = l.leitura}
          {#if leitura.estado === "lida" || (leitura.estado === "lista_vazia" && leitura.conexao)}
            {@const c = leitura.conexao!}
            <p class="destaque">Wi-Fi “{c.ssid}”, sinal {NOME_DO_SINAL[c.sinal]}</p>
            <dl class="ficha">
              <dt>Sinal</dt>
              <dd>{c.rssi_dbm} dBm</dd>
              {#if c.banda}<dt>Faixa</dt><dd>{c.banda}{c.canal != null ? `, canal ${c.canal}` : ""}</dd>{/if}
              {#if c.padrao}<dt>Padrão</dt><dd>{c.padrao}</dd>{/if}
              <dt>Com o roteador</dt>
              <dd>{c.recepcao_mbps} Mb/s</dd>
              {#if leitura.estado === "lida" && leitura.vizinhanca}
                <dt>Vizinhos</dt>
                <dd>
                  {leitura.vizinhanca.mesmo_canal} no mesmo canal{leitura.vizinhanca.sobrepostas > 0
                    ? `, ${leitura.vizinhanca.sobrepostas} sobrepostos`
                    : ""}
                </dd>
              {/if}
            </dl>
          {:else if leitura.estado === "sem_permissao_de_localizacao"}
            <p class="destaque">Wi-Fi</p>
            <p class="apoio">
              O Windows só deixa ler as redes por perto com a localização ligada para apps da área de trabalho
              (Configurações › Privacidade › Localização).
            </p>
          {:else if leitura.estado === "nao_li"}
            <p class="destaque">Wi-Fi</p>
            <p class="apoio">Não consegui ler o Wi-Fi: {leitura.motivo}</p>
          {:else}
            <p class="destaque">Wi-Fi</p>
            <p class="apoio">A placa não informou a rede agora.</p>
          {/if}
          <p class="apoio">{l.placa}</p>
        {/if}
      </div>

      {#if wifi.achados.length > 0}
        <ul class="achados">
          {#each wifi.achados as a, i (i)}
            <li>
              <span class="ponto" data-tom={a.gravidade === "Atencao" ? "atencao" : a.gravidade === "Bom" ? "ok" : "neutro"}></span>
              <span>{a.frase}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>

  <!-- Rota -->
  <section class="secao" aria-labelledby="rede-rota">
    <h2 id="rede-rota">Onde está o problema</h2>
    <form
      class="medir"
      onsubmit={(e) => {
        e.preventDefault();
        void medir("destino");
      }}
    >
      <input
        class="campo"
        bind:value={destino}
        aria-label="Endereço ou nome para medir"
        placeholder="endereço ou nome"
        disabled={medindo !== null}
      />
      <button class="btn primario" type="submit" disabled={medindo !== null || destino.trim() === ""}>
        {medindo === "destino" ? "Medindo…" : "Medir rota"}
      </button>
      <button class="btn" type="button" disabled={medindo !== null} onclick={() => medir("jogo")}>
        {medindo === "jogo" ? "Medindo…" : "Servidor do jogo"}
      </button>
    </form>

    {#if medindo}
      <p class="apoio">Mandando 10 pacotes por salto, espalhados pelo segundo. Leva uns 10 s.</p>
    {:else if erroRota}
      <p class="erro">{erroRota}</p>
    {:else if rota}
      <div class="veredito">
        <span class="ponto" data-tom={rota.veredito.tipo === "limpa" ? "ok" : rota.veredito.tipo === "problema" ? "atencao" : "neutro"}></span>
        <div>
          <p class="destaque">{rota.frase}</p>
          {#if rota.observacao}<p class="apoio">{rota.observacao}</p>{/if}
        </div>
      </div>

      <table class="saltos">
        <thead>
          <tr>
            <th class="n">#</th>
            <th>Trecho</th>
            <th>Quem</th>
            <th class="n">Tempo</th>
            <th class="n">Variação</th>
            <th class="n">Perda</th>
          </tr>
        </thead>
        <tbody>
          {#each rota.saltos as s (s.numero)}
            <tr class:problema={s.numero === saltoDoProblema} class:mudo={s.trecho === "sem_resposta"}>
              <td class="n numero">{s.numero}</td>
              <td>{NOME_DO_TRECHO[s.trecho]}</td>
              <td class="quem" title={s.endereco ?? undefined}>{quem(s)}</td>
              <td class="n numero">{ms(s.mediana_ms)}</td>
              <td class="n numero">{ms(s.jitter_ms)}</td>
              <td class="n numero">{perda(s)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if rota.atribuicao}<p class="apoio atribuicao">{rota.atribuicao}</p>{/if}
    {/if}
  </section>

  <!-- Durante o jogo -->
  <section class="secao" aria-labelledby="rede-jogo">
    <h2 id="rede-jogo">Durante o jogo</h2>
    <ul class="lista">
      <li>
        <div class="opcao">
          <span id="rede-banda">Segurar a banda do segundo plano</span>
          <span class="apoio">{banda?.o_que_faz ?? "Lendo…"}</span>
          {#if banda?.impedimento && !banda.ligado}
            <span class="impedimento">
              <span class="ponto" data-tom="atencao"></span>{banda.impedimento}
              <button class="btn quieto" onclick={abrirComoAdministrador}>Abrir como administrador</button>
            </span>
          {/if}
          {#if banda?.agindo_em}
            <span class="agindo">
              <span class="ponto" data-tom="ok"></span>Agindo em {banda.agindo_em}{banda.politicas.length > 0
                ? `: ${banda.politicas.map(programa).join(", ")}`
                : ""}{banda.update_limitado ? " · Windows Update limitado" : ""}
            </span>
          {/if}
          {#if respostaBanda}
            {#each respostaBanda.linhas as l, i (i)}
              <span class="resposta" data-tipo={respostaBanda.tipo}>{l}</span>
            {/each}
          {/if}
          {#if erroBanda}<span class="erro">{erroBanda}</span>{/if}
        </div>
        <button
          class="interruptor"
          role="switch"
          aria-checked={banda?.ligado ?? false}
          aria-labelledby="rede-banda"
          disabled={!banda || mudandoBanda || (!banda.ligado && banda.impedimento !== null)}
          onclick={alternarBanda}
        ></button>
      </li>
      <li>
        <div class="opcao">
          <span id="rede-varredura">Wi-Fi sem procurar redes</span>
          <span class="apoio">
            O Windows procura redes de tempos em tempos, e isso pode dar picos de ping no Wi-Fi. Ligado, a procura
            para enquanto o jogo está aberto e volta quando ele fecha.
          </span>
          {#if varredura?.agindo_em}
            <span class="agindo"><span class="ponto" data-tom="ok"></span>Agindo em {varredura.agindo_em}</span>
          {:else if varredura?.observacao}
            <span class="apoio">{varredura.observacao}</span>
          {/if}
          {#if erroVarredura}<span class="erro">{erroVarredura}</span>{/if}
        </div>
        <button
          class="interruptor"
          role="switch"
          aria-checked={varredura?.ligado ?? false}
          aria-labelledby="rede-varredura"
          disabled={!varredura || mudandoVarredura}
          onclick={alternarVarredura}
        ></button>
      </li>
    </ul>
  </section>
</div>

<style>
  .destaque {
    font-weight: 500;
  }
  .erro {
    color: var(--perigo);
  }
  .ligacao {
    display: grid;
    gap: 6px;
  }
  .ficha {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 24px;
    margin: 4px 0;
  }
  .ficha dt {
    color: var(--tinta-2);
  }
  .ficha dd {
    margin: 0;
  }
  .achados {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .achados li,
  .veredito {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .achados .ponto,
  .veredito .ponto {
    transform: translateY(-1px);
  }
  .veredito > div {
    display: grid;
    gap: 2px;
  }
  .medir {
    display: flex;
    gap: 8px;
  }
  .medir .campo {
    flex: 1;
    min-width: 0;
  }
  .saltos {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .saltos th {
    padding: 0 8px 6px;
    text-align: left;
    font-weight: 500;
    font-size: var(--t-apoio);
    color: var(--tinta-2);
    border-bottom: 1px solid var(--linha);
  }
  .saltos td {
    padding: 7px 8px;
    border-bottom: 1px solid var(--linha);
  }
  .saltos .n {
    text-align: right;
    white-space: nowrap;
  }
  .saltos th:first-child,
  .saltos td:first-child {
    padding-left: 0;
    width: 24px;
  }
  .saltos tr:last-child td {
    border-bottom: 0;
  }
  .saltos .quem {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .saltos tr.mudo td {
    color: var(--tinta-3);
  }
  .saltos tr.problema td {
    background: var(--ambar-fundo);
  }
  .atribuicao {
    font-size: 11px;
  }
  .lista > li {
    align-items: flex-start;
    gap: 24px;
  }
  .opcao {
    flex: 1;
    display: grid;
    gap: 4px;
  }
  .opcao > span:first-child {
    font-weight: 500;
  }
  .opcao .apoio {
    max-width: 60ch;
  }
  .impedimento,
  .agindo {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .resposta {
    font-size: 13px;
    color: var(--tinta-2);
  }
  .resposta[data-tipo="erro"] {
    color: var(--perigo);
  }
</style>
