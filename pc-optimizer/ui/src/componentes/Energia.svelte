<script lang="ts">
  import Esqueleto from "./Esqueleto.svelte";
  // A área Energia: o plano OTIMIZA (ver antes o que muda, aplicar, conferir se continua no lugar, reparar e desfazer),
  // a energia de jogo (o processador sem dormir só com o jogo aberto), os limites escondidos que seguram o FPS e o
  // "por que não funcionou?". O plano do cliente nunca é tocado: o OTIMIZA é um plano à parte e desfazer reativa o
  // anterior. Desfazer nunca depende de licença.
  import { onMount } from "svelte";
  import EnergiaPorJogo from "./EnergiaPorJogo.svelte";
  import Processador from "./Processador.svelte";
  import {
    ponte,
    type DiagnosticoDeEnergia,
    type EnergiaDeJogo,
    type PassoDaCalibracao,
    type ResultadoDaCalibracao,
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

  // Calibração por candidatos: o Rust conduz o roteiro inteiro; aqui só o progresso e o resultado.
  let admin = $state<boolean | null>(null);
  let calibrando = $state(false);
  let cancelando = $state(false);
  let passo = $state<PassoDaCalibracao | null>(null);
  let calibracao = $state<ResultadoDaCalibracao | null>(null);
  let erroCalibracao = $state<string | null>(null);
  let avisoCalibracao = $state<string | null>(null);
  let aplicandoCalibrado = $state(false);

  async function calibrar() {
    calibrando = true;
    cancelando = false;
    erroCalibracao = null;
    avisoCalibracao = null;
    calibracao = null;
    passo = null;
    try {
      calibracao = await ponte.calibrarEnergia((p) => (passo = p));
    } catch (e) {
      erroCalibracao = String(e);
    } finally {
      calibrando = false;
      void lerVistoria();
    }
  }

  async function cancelarCalibracao() {
    cancelando = true;
    await ponte.cancelarCalibracao().catch(() => {});
  }

  async function aplicarCalibrado() {
    aplicandoCalibrado = true;
    try {
      avisoCalibracao = await ponte.aplicarCalibrado();
      if (calibracao) calibracao = { ...calibracao, pode_aplicar: false };
      void lerVistoria();
    } catch (e) {
      erroCalibracao = String(e);
    } finally {
      aplicandoCalibrado = false;
    }
  }

  type Tom = "ok" | "atencao" | "neutro";
  /** Variação em %: verde quando melhora (no P99, menor é melhor), âmbar quando piora, neutra abaixo de meio ponto. */
  function sinal(v: number | null, menorEMelhor = false): { texto: string; tom: Tom } {
    if (v == null) return { texto: "—", tom: "neutro" };
    const bom = menorEMelhor ? v < 0 : v > 0;
    return {
      texto: `${v > 0 ? "+" : ""}${v.toFixed(1).replace(".", ",")}%`,
      tom: Math.abs(v) < 0.5 ? "neutro" : bom ? "ok" : "atencao",
    };
  }

  async function lerVistoria() {
    vistoria = await ponte.vistoriarPlano().catch(() => ({ estado: "NaoConsegui" }) as VistoriaDoPlano);
  }

  onMount(() => {
    void lerVistoria();
    ponte.administrador().then((a) => (admin = a)).catch(() => (admin = null));
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

  <!-- Calibração por candidatos -->
  <section class="secao">
    <h2>Calibrar para este PC</h2>
    <p class="apoio">
      O Otimiza testa de 4 a 8 combinações de energia no seu processador, mede cada uma e fica com a que der mais FPS e
      melhor 1% pior, sem esquentar à toa. Leva de 2 a 5 minutos. Com um jogo aberto, mede com ele (fique na mesma cena);
      sem jogo, deixe o PC parado. No fim, o seu plano volta: o vencedor só entra se você aplicar.
    </p>
    {#if admin === false}
      <p class="impedimento">
        A calibração troca o plano de energia: precisa do Otimiza aberto como administrador.
        <button class="btn quieto" onclick={() => ponte.abrirComoAdministrador()}>Abrir como administrador</button>
      </p>
    {/if}
    <div class="acoes">
      {#if calibrando}
        <button class="btn" disabled={cancelando} onclick={cancelarCalibracao}>
          {cancelando ? "Parando no fim do passo…" : "Parar"}
        </button>
      {:else}
        <button class="btn primario" disabled={admin === false} onclick={calibrar}>
          {calibracao ? "Calibrar de novo" : "Calibrar agora"}
        </button>
      {/if}
    </div>

    {#if calibrando}
      <div class="calibrando">
        <div class="progresso" role="progressbar" aria-valuemin={0} aria-valuemax={passo?.total ?? 1} aria-valuenow={passo?.passo ?? 0}>
          <div style:width={`${passo ? Math.min(100, (passo.passo / passo.total) * 100) : 3}%`}></div>
        </div>
        <p class="apoio">{passo?.frase ?? "Preparando…"}</p>
      </div>
    {:else if erroCalibracao}
      <p class="erro">{erroCalibracao}</p>
    {:else if calibracao}
      {@const c = calibracao}
      <div class="veredito-calibracao">
        <span class="ponto" data-tom={c.pode_aplicar ? "ok" : "neutro"}></span>
        <div>
          <p class="destaque">
            {#if c.fica_o_atual}
              O plano que você já usa é o melhor neste PC. Nada a trocar.
            {:else if c.vencedor && c.pode_aplicar}
              Ganhou: {c.vencedor} (confiança de {Math.round(c.confianca_pct ?? 0)}%).
            {:else if c.vencedor}
              Ganhou: {c.vencedor}.
            {:else}
              Nenhuma combinação ganhou do padrão do Windows com folga. Fica o padrão.
            {/if}
          </p>
          {#if !c.mediu_com_jogo}<p class="apoio">Medido sem jogo. Para a palavra final, calibre com o jogo aberto.</p>{/if}
        </div>
      </div>
      {#if c.vencedor && !c.fica_o_atual}
        {@const ganhos = [
          { rotulo: "FPS", g: sinal(c.fps_pct) },
          { rotulo: "1% pior", g: sinal(c.low_1_pct) },
          { rotulo: "pior quadro (P99)", g: sinal(c.p99_pct, true) },
        ]}
        <div class="ganhos-calibracao">
          {#each ganhos as x (x.rotulo)}
            <div><strong class="numero" data-tom={x.g.tom}>{x.g.texto}</strong><span class="apoio">{x.rotulo}</span></div>
          {/each}
          {#if c.temperatura_delta_c != null}
            <div>
              <strong class="numero">{c.temperatura_delta_c > 0 ? "+" : ""}{c.temperatura_delta_c.toFixed(1).replace(".", ",")} °C</strong>
              <span class="apoio">temperatura</span>
            </div>
          {/if}
        </div>
      {/if}
      <ol class="notas">
        {#each c.candidatos as n, i (i)}
          <li class:vencedor={n.vencedor}>
            <span class="rot">{n.rotulo}</span>
            <span class="barra-nota" aria-hidden="true"><span style:width={`${Math.max(4, Math.min(100, ((n.nota ?? 100) - 80) * 2.5))}%`}></span></span>
            <span class="numero">{(n.nota ?? 0).toFixed(1).replace(".", ",")}</span>
          </li>
        {/each}
      </ol>
      <p class="apoio pequeno">Nota 100 = igual ao padrão do Windows. Pesam mais o 1% pior, o pior quadro e a resposta do processador.</p>
      {#if c.achados.length > 0}
        <ul class="achados-calibracao">
          {#each c.achados as a, i (i)}
            <li><span class="ponto" data-tom={a.tom}></span><span>{a.texto}</span></li>
          {/each}
        </ul>
      {/if}
      {#if c.aviso}<p class="impedimento">{c.aviso}</p>{/if}
      {#if c.pode_aplicar}
        <div class="acoes">
          <button class="btn primario" disabled={aplicandoCalibrado} onclick={aplicarCalibrado}>
            {aplicandoCalibrado ? "Aplicando…" : `Aplicar ${c.vencedor}`}
          </button>
        </div>
      {/if}
      {#if avisoCalibracao}<p class="apoio">{avisoCalibracao}</p>{/if}
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
      <Esqueleto linhas={2} rotulo="Procurando" />
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
  <EnergiaPorJogo />
  <Processador />
</div>

<style>
  .energia {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }
  .calibrando {
    display: grid;
    gap: 8px;
  }
  .progresso {
    height: 6px;
    border-radius: 3px;
    background: var(--linha);
    overflow: hidden;
  }
  .progresso div {
    height: 100%;
    background: var(--acao);
    transition: width 600ms ease;
  }
  .veredito-calibracao {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .veredito-calibracao p {
    margin: 0;
  }
  .destaque {
    font-weight: 500;
  }
  .erro {
    color: var(--perigo);
  }
  .ganhos-calibracao {
    display: flex;
    gap: 28px;
    flex-wrap: wrap;
  }
  .ganhos-calibracao > div {
    display: grid;
    gap: 2px;
  }
  .ganhos-calibracao .numero {
    font-size: 22px;
  }
  .ganhos-calibracao [data-tom="ok"] {
    color: var(--ok);
  }
  .ganhos-calibracao [data-tom="atencao"] {
    color: var(--atencao);
  }
  .notas {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .notas li {
    display: grid;
    grid-template-columns: minmax(140px, 220px) 1fr 48px;
    gap: 12px;
    align-items: center;
    font-size: 13px;
  }
  .notas li.vencedor .rot {
    font-weight: 600;
  }
  .barra-nota {
    height: 8px;
    border-radius: 4px;
    background: var(--linha);
    overflow: hidden;
  }
  .barra-nota span {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--tinta-3);
  }
  .notas li.vencedor .barra-nota span {
    background: var(--ok);
  }
  .notas .numero {
    text-align: right;
  }
  .pequeno {
    font-size: 11px;
  }
  .achados-calibracao {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
    font-size: 13px;
  }
  .achados-calibracao li {
    display: flex;
    gap: 8px;
    align-items: baseline;
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
