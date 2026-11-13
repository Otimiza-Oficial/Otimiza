<script lang="ts">
  import { ferramentas } from "../lib/ferramentas";
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import Esqueleto from "./Esqueleto.svelte";
  // A área Ajustes: o FPS por cima do jogo (sobreposição), o que o Otimiza faz sozinho, a licença, a versão e o
  // relatório de suporte. O tema (claro/escuro) mora na barra de cima, onde a referência põe.
  import { onMount } from "svelte";
  import Transparencia from "./Transparencia.svelte";
  import { ABRIR_FEEDBACK } from "./PedidoDeFeedback.svelte";
  import AtivarLicenca from "./AtivarLicenca.svelte";
  import { ABRIR_BOAS_VINDAS } from "./BoasVindas.svelte";
    import { noTauri } from "../lib/ponte";
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
  import { temporadaDe } from "../lib/temporadas";

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
  /** O Smooth Motion pelo perfil do driver (`flags.smooth_motion_drs`): experimental, só em placa NVIDIA, desligado
   *  de fábrica. As outras chaves de `flags` ficam como estão: lê tudo e grava tudo, trocando só esta. */
  let nvidia = $state(false);
  let smoothMotion = $state<boolean | null>(null);
  /** A escala do Frame Boost (o jogo com menos pixels, ampliado para a tela): desligada por padrão. */
  let escalaPermitida = $state<boolean | null>(null);
  async function alternarEscala() {
    if (escalaPermitida === null) return;
    try {
      escalaPermitida = await ferramentas.definirEscalaPermitida(!escalaPermitida);
    } catch (e) {
      recado = { tom: "perigo", texto: String(e) };
    }
  }
  async function alternarSmoothMotion() {
    if (!noTauri || smoothMotion === null) return;
    try {
      const atual = await como<Record<string, unknown>>(commands.flagsLer());
      const ligado = !atual.smooth_motion_drs;
      await commands.flagsGravar({ ...atual, smooth_motion_drs: ligado });
      smoothMotion = ligado;
    } catch (e) {
      recado = { tom: "perigo", texto: String(e) };
    }
  }

  /** Só a situação do pedido de feedback (a folha que resolve mora em `PedidoDeFeedback`). */
  let feedback = $state<
    { situacao: "NaoSeAplica" } | { situacao: "Contando"; restante_segundos: number } | { situacao: "Liberado"; verificado_em: string } | { situacao: "Exigido"; apagado: boolean } | null
  >(null);
  let aviso = $state<AvisoDeVersao | null>(null);
  let versao = $state<VersaoDoApp | null>(null);
  let recado = $state<{ tom: "ok" | "perigo" | "atencao"; texto: string } | null>(null);

  let ia = $state(lerIa());

  function lerIa() {
    try {
      return localStorage.getItem("otimiza.ia") === "ligada";
    } catch {
      return false;
    }
  }

  function alternarIa() {
    ia = !ia;
    try {
      localStorage.setItem("otimiza.ia", ia ? "ligada" : "desligada");
    } catch {
      // Só não lembra na próxima abertura.
    }
  }

  let epoca = $state(lerEpoca());
  const epocaDeHoje = temporadaDe(new Date());

  function lerEpoca() {
    try {
      return localStorage.getItem("otimiza.epoca") !== "desligada";
    } catch {
      return true;
    }
  }

  function alternarEpoca() {
    epoca = !epoca;
    try {
      localStorage.setItem("otimiza.epoca", epoca ? "ligada" : "desligada");
    } catch {
      // Só não lembra na próxima abertura.
    }
    // A casca escuta e troca na hora.
    window.dispatchEvent(new Event("otimiza.epoca"));
  }

  let discordDisponivel = $state(false);
  let mudandoComunidade = $state(false);
  let nuvem = $state<{ tipo: "ok" | "erro"; texto: string } | null>(null);
  let nuvemOcupada = $state(false);

  async function alternarComunidade() {
    if (!preferencias) return;
    mudandoComunidade = true;
    try {
      const ligado = await ponte.comunidadeParticipar(!preferencias.comunidade);
      preferencias = { ...preferencias, comunidade: ligado };
    } catch (e) {
      nuvem = { tipo: "erro", texto: String(e) };
    } finally {
      mudandoComunidade = false;
    }
  }

  async function usarNuvem(qual: "guardar" | "restaurar") {
    if (qual === "restaurar" && !confirm("Trazer da nuvem o histórico e as preferências? O que está aqui hoje fica guardado ao lado.")) return;
    nuvemOcupada = true;
    nuvem = null;
    try {
      nuvem = { tipo: "ok", texto: qual === "guardar" ? await ponte.nuvemGuardar() : await ponte.nuvemRestaurar() };
    } catch (e) {
      nuvem = { tipo: "erro", texto: String(e) };
    } finally {
      nuvemOcupada = false;
    }
  }

  onMount(() => {
    ponte.discordDisponivel().then((d) => (discordDisponivel = d)).catch(() => {});
    ponte.sobreposicao().then((s) => (sobreposicao = s)).catch(() => {});
    ponte.preferencias().then((p) => (preferencias = p)).catch(() => {});
    ponte.licenca().then((l) => (licenca = l)).catch(() => {});
    if (noTauri) como<NonNullable<typeof feedback>>(commands.feedbackEstado()).then((f) => (feedback = f)).catch(() => {});
    ponte
      .hardwareUltimo()
      .then((h) => {
        nvidia = h?.placa.fabricante === "Nvidia";
        ferramentas.escalaPermitida().then((v) => (escalaPermitida = v)).catch(() => (escalaPermitida = false));
        if (nvidia && noTauri) como<{ smooth_motion_drs: boolean }>(commands.flagsLer()).then((f) => (smoothMotion = f.smooth_motion_drs)).catch(() => {});
      })
      .catch(() => {});
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
        <div class="segmento cantos" role="radiogroup" aria-label="Canto da tela">
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
          <div class="info">
            <span>Mostrar o jogo no Discord</span>
            <span class="apoio">
              {discordDisponivel
                ? "Seu perfil mostra \"Jogando <jogo> · com o Otimiza\" com o Oti, pelo Discord aberto neste PC. Só o nome do jogo sai daqui."
                : "Não disponível nesta versão."}
            </span>
          </div>
          <button class="interruptor" role="switch" aria-checked={preferencias.discord_presenca} aria-label="Mostrar o jogo no Discord" disabled={!discordDisponivel} onclick={() => mudar("discord_presenca", !preferencias!.discord_presenca)}></button>
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

  <!-- Oti com IA -->
  <section class="secao">
    <div class="linha">
      <div class="info">
        <h2>Oti com IA</h2>
        <span class="apoio">
          Quando o Oti não entende uma pergunta, ela vai para a IA, com os números do seu PC (processador, placa,
          memória, Hz, FPS da última partida e avisos). Nada de arquivo, nome de usuário ou senha. A IA só responde e
          sugere qual área abrir: quem muda algo no PC é você. Precisa de licença; tem um limite de perguntas por dia.
        </span>
      </div>
      <button class="interruptor" role="switch" aria-checked={ia} aria-label="Oti com IA" onclick={alternarIa}></button>
    </div>
  </section>

  <!-- Comunidade e nuvem -->
  <section class="secao">
    {#if preferencias}
      <div class="linha">
        <div class="info">
          <h2>Comunidade do Otimiza</h2>
          <span class="apoio">
            Depois de cada partida medida, manda de forma anônima o nome do processador e da placa, o jogo e o FPS; e o
            plano que ganhou na calibração. Em troca você vê o que mais ganhou em PCs iguais ao seu, e os números entram
            na bancada pública do site. Nada sai com menos de 3 PCs iguais. Desligar apaga o que você mandou.
          </span>
        </div>
        <button class="interruptor" role="switch" aria-checked={preferencias.comunidade} aria-label="Comunidade do Otimiza" disabled={mudandoComunidade} onclick={alternarComunidade}></button>
      </div>
    {/if}
    <div class="linha">
      <div class="info">
        <h2>Backup na nuvem</h2>
        <span class="apoio">
          Guarda o histórico de partidas, de rede e as suas preferências na sua licença. Formatou ou trocou de PC? Traga
          de volta. O estado das mudanças deste PC não vai junto: noutro PC ele não valeria.
        </span>
      </div>
      <div class="botoes-nuvem">
        <button class="btn" disabled={nuvemOcupada} onclick={() => usarNuvem("guardar")}>Guardar</button>
        <button class="btn" disabled={nuvemOcupada} onclick={() => usarNuvem("restaurar")}>Restaurar</button>
      </div>
    </div>
    {#if nuvem}<p class="apoio" class:erro-nuvem={nuvem.tipo === "erro"}>{nuvem.texto}</p>{/if}
  </section>

  <!-- Época do ano -->
  <section class="secao">
    <div class="linha">
      <div class="info">
        <h2>Temas de época</h2>
        <span class="apoio">
          O painel muda sozinho no Natal, Ano Novo, Carnaval, Páscoa, Festa Junina, Dia das Crianças e Halloween: o Oti
          ganha um enfeite e o fundo, um clima. Para de animar com jogo aberto.
          {#if epocaDeHoje}Agora: <b>{epocaDeHoje.nome}</b>.{/if}
        </span>
      </div>
      <button class="interruptor" role="switch" aria-checked={epoca} aria-label="Temas de época" onclick={alternarEpoca}></button>
    </div>
  </section>

  <!-- A resolução do jogo -->
  {#if escalaPermitida !== null}
    <section class="secao">
      <div class="linha">
        <div class="info">
          <h2>Permitir a escala</h2>
          <span class="apoio">
            Com ela, o Frame Boost e o Turbo podem deixar o jogo numa janela menor (menos pixels, mais FPS) e ampliar para a
            tela. Desligada, o Otimiza nunca muda a resolução do jogo: o Frame Boost só gera quadros.
          </span>
        </div>
        <button class="interruptor" role="switch" aria-checked={escalaPermitida} aria-label="Permitir a escala" onclick={alternarEscala}></button>
      </div>
    </section>
  {/if}

  {#if nvidia && smoothMotion !== null}
    <!-- Experimental -->
    <section class="secao">
      <div class="linha">
        <div class="info">
          <h2>Smooth Motion pelo Otimiza <span class="selo">experimental</span></h2>
          <span class="apoio">
            Liga o Smooth Motion da NVIDIA no perfil do jogo no driver, com um ajuste que a NVIDIA não documenta. Só RTX 40 e
            50; desliga num clique.
          </span>
        </div>
        <button class="interruptor" role="switch" aria-checked={smoothMotion} aria-label="Smooth Motion pelo Otimiza" onclick={alternarSmoothMotion}></button>
      </div>
    </section>
  {/if}

  <!-- Licença -->
  <section class="secao">
    <h2>Licença</h2>
    {#if !licenca}
      <Esqueleto linhas={2} />
    {:else if licenca.ativa}
      <ul class="lista">
        <li><span class="info">{licenca.origem}</span><span class="apoio">{licenca.expira ? `até ${licenca.expira}` : "sem vencimento"}</span></li>
        {#if licenca.comprador}<li><span class="info">Em nome de</span><span>{licenca.comprador}</span></li>{/if}
        <li>
          <span class="info">Esta máquina</span><span class="numero">{licenca.maquina}</span>
        </li>
        {#if feedback && feedback.situacao !== "NaoSeAplica"}
          <li>
            <span class="info" class:atencao={feedback.situacao === "Exigido"}>
              Feedback
              <span class="apoio">
                {feedback.situacao === "Liberado"
                  ? "Recebido. Obrigado."
                  : feedback.situacao === "Exigido"
                    ? "As correções estão pausadas até chegar o seu feedback."
                    : `O pedido aparece em ${Math.max(1, Math.ceil(feedback.restante_segundos / 60))} min.`}
              </span>
            </span>
            {#if feedback.situacao === "Exigido"}<button class="btn" onclick={() => window.dispatchEvent(new Event(ABRIR_FEEDBACK))}>Enviar feedback</button>{/if}
          </li>
        {/if}
        <li>
          <span class="info" class:atencao={!licenca.sobrevive_formatacao}>{licenca.sobrevive_formatacao ? "Continua valendo se você formatar o PC" : "Formatar o PC pede ativar de novo"}</span>
        </li>
      </ul>
    {:else}
      <p class="apoio">{licenca.motivo ?? "Sem licença: medir, diagnosticar e desfazer continuam livres."}</p>
      <AtivarLicenca {licenca} onativada={(l) => (licenca = l)} />
    {/if}
    <button class="btn quieto" onclick={() => window.dispatchEvent(new CustomEvent(ABRIR_BOAS_VINDAS, { detail: "completa" }))}>Ver a apresentação de novo</button>
  </section>

  <!-- Versão e suporte -->
  <section class="secao">
    <h2>Versão e suporte</h2>
    <ul class="lista">
      <li>
        <span class="info">
          Otimiza {versao?.exibida ?? ""}
          {#if versao && !versao.oficial}
            <span class="apoio">Compilação local: sem a conferência online da licença. Para cliente, use o instalador publicado.</span>
          {/if}
          <span class="apoio">
            {aviso?.comparacao === "HaVersaoNova"
              ? `A ${aviso.versao_publicada_exibida ?? aviso.versao_publicada} já saiu`
              : aviso?.comparacao === "NaoSei"
                ? "Não consegui conferir se há versão nova"
                : "É a mais nova"}
          </span>
        </span>
        {#if aviso?.comparacao === "HaVersaoNova" && aviso.pagina}
          <a class="btn" href={aviso.pagina} target="_blank" rel="noreferrer">Ver a versão nova</a>
        {/if}
      </li>
      <li>
        <span class="info">Relatório para o suporte<span class="apoio">O que o atendimento precisa, sem dados pessoais</span></span>
        <button class="btn" onclick={copiarRelatorio}>Copiar</button>
      </li>
    </ul>
  </section>
  <Transparencia />
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
  .info.atencao {
    color: var(--atencao);
  }
  .controles {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
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
  a.btn {
    text-decoration: none;
  }
  .botoes-nuvem {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .erro-nuvem {
    color: var(--perigo);
  }
  .cantos button {
    width: 34px;
    padding: 0;
    font-size: 15px;
  }
</style>
