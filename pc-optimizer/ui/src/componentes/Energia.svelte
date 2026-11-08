<script lang="ts">
  // A área Energia: o plano OTIMIZA (ver antes o que muda, aplicar, conferir se continua no lugar, reparar e desfazer),
  // a energia de jogo (o processador sem dormir só com o jogo aberto), os limites escondidos que seguram o FPS e o
  // "por que não funcionou?". O plano do cliente nunca é tocado: o OTIMIZA é um plano à parte e desfazer reativa o
  // anterior. Desfazer nunca depende de licença.
  import { onMount } from "svelte";
  import {
    ponte,
    type DiagnosticoDeEnergia,
    type EnergiaDeJogo,
    type RelatorioDeTetos,
    type RelatorioDoPlano,
    type VistoriaDoPlano,
  } from "../lib/ponte";

  const STATUS: Record<string, { texto: string; tom: "ok" | "atencao" | "perigo" | "neutro" }> = {
    Aplicado: { texto: "aplicado", tom: "ok" },
    JaEstavaBom: { texto: "já estava bom", tom: "neutro" },
    NaoSuportado: { texto: "não existe neste PC", tom: "neutro" },
    FalhouAoAplicar: { texto: "falhou", tom: "perigo" },
    FalhouNaVerificacao: { texto: "o Windows não guardou", tom: "perigo" },
    Mudaria: { texto: "vai mudar", tom: "atencao" },
    Pulado: { texto: "não se aplica aqui", tom: "neutro" },
  };

  let vistoria = $state<VistoriaDoPlano | null>(null);
  let relatorio = $state<RelatorioDoPlano | null>(null);
  let jogo = $state<EnergiaDeJogo | null>(null);
  let tetos = $state<RelatorioDeTetos | null>(null);
  let diagnostico = $state<DiagnosticoDeEnergia | null>(null);
  let verDiagnostico = $state(false);
  let ocupado = $state<"" | "simulando" | "aplicando" | "reparando" | "desfazendo" | "jogo">("");
  let erro = $state<string | null>(null);
  let aviso = $state<string | null>(null);

  async function lerVistoria() {
    vistoria = await ponte.vistoriarPlano().catch(() => ({ estado: "NaoConsegui" }) as VistoriaDoPlano);
  }

  onMount(() => {
    void lerVistoria();
    ponte.energiaDeJogo().then((j) => (jogo = j)).catch((e) => (erro = String(e)));
    ponte.tetosEscondidos().then((t) => (tetos = t)).catch(() => {});
  });

  async function executar(tipo: "simulando" | "aplicando" | "reparando", f: () => Promise<RelatorioDoPlano>) {
    ocupado = tipo;
    erro = null;
    aviso = null;
    try {
      relatorio = await f();
      if (!relatorio.simulacao) {
        aviso =
          relatorio.desfecho === "Sucesso"
            ? `Plano OTIMIZA ativo: ${relatorio.aplicados} ajustes aplicados, ${relatorio.ja_estavam_bons} já estavam bons.`
            : relatorio.desfecho === "EmParte"
              ? `Plano ativo, mas ${relatorio.falhas} ajustes não foram aceitos pelo Windows. Veja a lista.`
              : "O Windows não aceitou o plano. Veja a lista e o diagnóstico abaixo.";
      }
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = "";
      await lerVistoria();
    }
  }

  async function desfazer() {
    ocupado = "desfazendo";
    erro = null;
    try {
      await ponte.desfazerPlano();
      relatorio = null;
      aviso = "Pronto: o seu plano de energia voltou a ser o ativo.";
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = "";
      await lerVistoria();
    }
  }

  async function alternarJogo() {
    if (!jogo) return;
    ocupado = "jogo";
    try {
      jogo = await ponte.definirEnergiaDeJogo(!jogo.ligado);
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = "";
    }
  }

  async function abrirDiagnostico() {
    verDiagnostico = !verDiagnostico;
    if (verDiagnostico && !diagnostico) diagnostico = await ponte.diagnosticoDeEnergia().catch(() => null);
  }

  function valor(v: number | null) {
    return v == null ? "—" : String(v);
  }

  function frase(t: RelatorioDeTetos["tetos"][number]): string {
    switch (t.tipo) {
      case "LimiteGlobalNvidia":
        return `O painel da NVIDIA limita todos os jogos a ${t.fps} FPS.`;
      case "VsyncForcadoNvidia":
        return "O painel da NVIDIA força a sincronia vertical: o FPS fica preso aos Hz do monitor.";
      case "Rtss":
        return "O RivaTuner (RTSS) está aberto e pode estar limitando o FPS.";
      case "LimiteNoJogo":
        return `${t.jogo} está limitado a ${t.fps} FPS na configuração dele.`;
      case "VsyncNoJogo":
        return `${t.jogo} está com sincronia vertical ligada na configuração dele.`;
      case "RobloxLimitado":
        return `O Roblox está preso em ${t.fps ?? 60} FPS (o padrão de fábrica), abaixo do seu monitor.`;
    }
  }
</script>

<div class="energia">
  {#if erro}
    <div class="recado" role="alert"><span class="ponto" data-tom="perigo"></span><div><span>{erro}</span></div></div>
  {/if}
  {#if aviso}
    <div class="recado" role="status"><span class="ponto" data-tom="ok"></span><div><span>{aviso}</span></div></div>
  {/if}

  <!-- Plano OTIMIZA -->
  <section class="secao">
    <div class="linha-opcao">
      <div>
        <h2>Plano de energia OTIMIZA</h2>
        <span class="apoio">
          {#if vistoria === null}
            Conferindo…
          {:else if vistoria.estado === "Integro"}
            Ativo e do jeito que o Otimiza deixou.
          {:else if vistoria.estado === "NaoExiste"}
            Um plano à parte, só para jogo. O seu plano continua guardado e volta com um toque.
          {:else if vistoria.estado === "DesativadoPorFora"}
            Outro programa (driver, utilitário do fabricante) trocou o plano ativo sem avisar.
          {:else if vistoria.estado === "Desviado"}
            Alguém mexeu em {vistoria.ajustes.length} ajustes do plano: {vistoria.ajustes.join(", ")}.
          {:else}
            Não consegui conferir o plano agora.
          {/if}
        </span>
      </div>
      <div class="acoes">
        {#if vistoria?.estado === "Integro"}
          <button class="btn" disabled={ocupado !== ""} onclick={desfazer}>{ocupado === "desfazendo" ? "Desfazendo…" : "Desfazer"}</button>
        {:else if vistoria?.estado === "DesativadoPorFora" || vistoria?.estado === "Desviado"}
          <button class="btn primario" disabled={ocupado !== ""} onclick={() => executar("reparando", ponte.repararPlano)}>
            {ocupado === "reparando" ? "Reparando…" : "Reparar"}
          </button>
        {:else}
          <button class="btn" disabled={ocupado !== ""} onclick={() => executar("simulando", ponte.simularPlano)}>
            {ocupado === "simulando" ? "Lendo…" : "Ver o que muda"}
          </button>
          <button class="btn primario" disabled={ocupado !== ""} onclick={() => executar("aplicando", ponte.aplicarPlano)}>
            {ocupado === "aplicando" ? "Aplicando…" : "Aplicar"}
          </button>
        {/if}
      </div>
    </div>

    {#if relatorio}
      <div class="contagem">
        {#if relatorio.simulacao}
          <span><b class="numero">{relatorio.ajustes.filter((a) => a.status === "Mudaria").length}</b> vão mudar</span>
        {:else}
          <span><b class="numero">{relatorio.aplicados}</b> aplicados</span>
        {/if}
        <span><b class="numero">{relatorio.ja_estavam_bons}</b> já estavam bons</span>
        <span><b class="numero">{relatorio.nao_suportados}</b> não existem neste PC</span>
        {#if relatorio.falhas > 0}<span class="ruim"><b class="numero">{relatorio.falhas}</b> falharam</span>{/if}
      </div>
      <table class="ajustes">
        <thead>
          <tr><th>Ajuste</th><th class="n">Antes</th><th class="n">{relatorio.simulacao ? "Vai para" : "Agora"}</th><th>Situação</th></tr>
        </thead>
        <tbody>
          {#each relatorio.ajustes.filter((a) => a.status !== "JaEstavaBom" && a.status !== "Pulado") as a (a.nome)}
            {@const st = STATUS[a.status] ?? { texto: a.status, tom: "neutro" }}
            <tr>
              <td><strong>{a.nome}</strong><span class="apoio porque">{a.porque}</span></td>
              <td class="n numero">{valor(a.ac_antes)}</td>
              <td class="n numero">{valor(relatorio.simulacao ? a.ac_alvo : (a.ac_depois ?? a.ac_alvo))}</td>
              <td><span class="situacao"><span class="ponto" data-tom={st.tom}></span>{st.texto}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <!-- Energia de jogo -->
  <section class="secao">
    <div class="linha-opcao">
      <div>
        <h2>Energia só durante o jogo</h2>
        <span class="apoio">{jogo?.o_que_muda ?? "Lendo…"}</span>
        {#if jogo?.agindo_em}<span class="agindo"><span class="ponto" data-tom="ok"></span>Agindo agora em {jogo.agindo_em}</span>{/if}
        {#if jogo?.impedimento && !jogo.ligado}<span class="impedimento">{jogo.impedimento}</span>{/if}
      </div>
      <button
        class="interruptor"
        role="switch"
        aria-checked={jogo?.ligado ?? false}
        aria-label="Energia só durante o jogo"
        disabled={!jogo || ocupado === "jogo" || (!jogo.ligado && jogo.impedimento !== null)}
        onclick={alternarJogo}
      ></button>
    </div>
  </section>

  <!-- Limites escondidos -->
  <section class="secao">
    <h2>Limites escondidos</h2>
    {#if tetos === null}
      <p class="apoio">Procurando…</p>
    {:else if tetos.tetos.length === 0}
      <p class="apoio">Nenhum limite de FPS escondido no driver, no RivaTuner ou na configuração dos jogos que eu sei ler.</p>
    {:else}
      <ul class="lista">
        {#each tetos.tetos as t, i (i)}
          <li><span class="ponto" data-tom="atencao"></span><span>{frase(t)}</span></li>
        {/each}
      </ul>
    {/if}
    {#if tetos && tetos.lacunas.length > 0}
      <p class="apoio">Não consegui ler: {tetos.lacunas.join("; ")}.</p>
    {/if}
  </section>

  <!-- Diagnóstico -->
  <section class="secao">
    <button class="btn quieto" aria-expanded={verDiagnostico} onclick={abrirDiagnostico}>
      {verDiagnostico ? "Esconder" : "Por que não funcionou?"}
    </button>
    {#if verDiagnostico}
      {#if !diagnostico}
        <p class="apoio">Conferindo o Windows…</p>
      {:else}
        <p>{diagnostico.explicacao_do_governo}</p>
        <ul class="lista">
          <li><span class="ponto" data-tom={diagnostico.elevado ? "ok" : "atencao"}></span><span class="rot">Otimiza como administrador</span><span>{diagnostico.elevado ? "sim" : "não"}</span></li>
          <li><span class="ponto" data-tom={diagnostico.powercfg_responde ? "ok" : "perigo"}></span><span class="rot">O Windows responde sobre energia</span><span>{diagnostico.powercfg_responde ? "sim" : "não"}</span></li>
          <li>
            <span class="ponto" data-tom={diagnostico.ajustes_suportados === diagnostico.ajustes_totais ? "ok" : "neutro"}></span>
            <span class="rot">Ajustes que este PC tem</span><span class="numero">{diagnostico.ajustes_suportados} de {diagnostico.ajustes_totais}</span>
          </li>
          <li><span class="ponto"></span><span class="rot">Processador</span><span>{diagnostico.maquina.cpu}{diagnostico.maquina.notebook ? " · notebook" : ""}</span></li>
        </ul>
        {#each diagnostico.avisos as a, i (i)}
          <p class="apoio">{a}</p>
        {/each}
        {#if diagnostico.ajustes_ausentes.length > 0}
          <p class="apoio">Não existem aqui: {diagnostico.ajustes_ausentes.join("; ")}.</p>
        {/if}
      {/if}
    {/if}
  </section>
</div>

<style>
  .energia {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }
  .linha-opcao {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
  }
  .linha-opcao > div:first-child {
    display: grid;
    gap: 4px;
    max-width: 62ch;
  }
  .acoes {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .contagem {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    font-size: 13px;
    color: var(--tinta-2);
  }
  .contagem b {
    color: var(--tinta);
    font-size: 16px;
  }
  .contagem .ruim b {
    color: var(--perigo);
  }
  .ajustes {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .ajustes th {
    padding: 0 8px 6px 0;
    text-align: left;
    font-weight: 500;
    font-size: var(--t-apoio);
    color: var(--tinta-2);
    border-bottom: 1px solid var(--linha);
  }
  .ajustes td {
    padding: 9px 8px 9px 0;
    border-bottom: 1px solid var(--linha);
    vertical-align: top;
  }
  .ajustes tr:last-child td {
    border-bottom: 0;
  }
  .ajustes .n {
    text-align: right;
    white-space: nowrap;
  }
  .porque {
    display: block;
    margin-top: 2px;
  }
  .situacao {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .agindo,
  .impedimento {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .impedimento {
    color: var(--atencao);
  }
  .secao > .btn.quieto {
    justify-self: start;
  }
  .lista .rot {
    flex: 1;
  }
</style>
