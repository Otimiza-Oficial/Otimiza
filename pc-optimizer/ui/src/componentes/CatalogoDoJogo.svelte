<script lang="ts">
  // O catálogo de um jogo no destaque da aba Jogos (a lista em `lib/catalogo.ts`): a configuração do próprio jogo (o
  // perfil de `configmotor.rs`, só chaves que já existem, com o jogo fechado e desfazer byte a byte), os caches de
  // shader que este jogo usa (os do driver e, quando há, o da Steam) e o que o anticheat deixa o Otimiza fazer. Três
  // cartões soltos: quem chama (a aba Otimizar do jogo) põe na grade. O FiveM entra com o que é dele pelos encaixes
  // `configExtra` (os gráficos do GTA) e `cacheExtra` (as pastas do FiveM), no mesmo cartão em vez de outra seção.
  import { onMount, type Snippet } from "svelte";
  import Cartao from "./base/Cartao.svelte";
  import Esqueleto from "./base/Esqueleto.svelte";
  import Estado from "./base/Estado.svelte";
  import type { JogoDoCatalogo } from "../lib/catalogo";
  import { como } from "../lib/contrato";
  import { confirmar } from "../lib/confirmar";
  import { commands } from "../lib/tipos";
  import { noTauri } from "../lib/ponte";

  let { jogo, configExtra, cacheExtra }: { jogo: JogoDoCatalogo; configExtra?: Snippet; cacheExtra?: Snippet } = $props();

  type Previsao = { chave: string; antes: string; depois: string; por_que: string };
  type Config = { id: string; nome: string; arquivo: string | null; aberto: boolean; previsao: Previsao[]; ilegivel: string | null; historico: string; aplicado: boolean };
  type Cache = { id: string; name: string; bytes: number; formatted: string; stale: boolean };

  const demo = (): { config: Config | null; caches: Cache[] } => ({
    config: jogo.perfil
      ? {
          id: jogo.perfil, nome: jogo.nome, arquivo: "GameUserSettings.ini", aberto: false, ilegivel: null,
          historico: `config_motor_${jogo.perfil}`, aplicado: false,
          previsao: [{ chave: "sg.ShadowQuality", antes: "2", depois: "0", por_que: "Sombras no mínimo: das opções mais caras." }],
        }
      : null,
    caches: [{ id: "nvidia_dx", name: "Cache do DirectX (NVIDIA)", bytes: 9.1e8, formatted: "910 MB", stale: true }],
  });

  let config = $state<Config | null>(null);
  let caches = $state<Cache[]>([]);
  /** "Não consegui ler" não é "não há nada": cada leitura guarda se falhou. */
  let lendo = $state(true);
  let configFalhou = $state(false);
  let cacheFalhou = $state(false);
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);
  let recadoEm = $state<"config" | "cache">("config");

  async function carregar() {
    if (!noTauri) {
      ({ config, caches } = demo());
      lendo = false;
      return;
    }
    const [c, s] = await Promise.allSettled([
      jogo.perfil ? como<Config[]>(commands.configMotorPainel()) : Promise.resolve([] as Config[]),
      como<{ caches: Cache[] }>(commands.analyzeShaders()),
    ]);
    config = c.status === "fulfilled" ? (c.value.find((x) => x.id === jogo.perfil) ?? null) : null;
    configFalhou = c.status === "rejected" || (!!jogo.perfil && config === null);
    caches = s.status === "fulfilled" ? s.value.caches.filter((x) => jogo.caches.includes(x.id) && x.bytes > 0) : [];
    cacheFalhou = s.status === "rejected";
    lendo = false;
  }

  onMount(carregar);

  async function fazer(chave: string, acao: () => Promise<string>) {
    ocupado = chave;
    recado = null;
    recadoEm = chave === "config" ? "config" : "cache";
    try {
      recado = await acao();
      await carregar();
    } catch (e) {
      recado = `Não deu certo: ${String(e)}. Feche o jogo, confira se o Otimiza está como administrador e tente de novo.`;
    } finally {
      ocupado = null;
    }
  }

  const aplicar = () =>
    fazer("config", async () => {
      if (!noTauri) return "Demonstração: nada foi gravado.";
      if (!jogo.perfil) return "Este jogo não tem perfil leve no Otimiza.";
      const mudou = await como<string[]>(commands.configMotorAplicar(jogo.perfil));
      return mudou.length
        ? `Gravei ${mudou.length} ${mudou.length === 1 ? "ajuste" : "ajustes"} na configuração do ${jogo.nome}. Abra o jogo e veja no menu de vídeo; o desfazer fica aqui e no Histórico.`
        : "A configuração já estava assim: nada a mudar.";
    });

  const desfazer = (c: Config) =>
    fazer("config", async () => {
      if (noTauri) await como(commands.revertOptimization(c.historico));
      return `A configuração do ${jogo.nome} voltou como estava.`;
    });

  const limpar = async (x: Cache) => {
    if (!(await confirmar({ titulo: `Apagar ${x.name}?`, texto: `Libera ${x.formatted}. A primeira partida recompila os shaders e pode engasgar; da segunda em diante fica mais liso.`, sim: "Apagar", perigo: true }))) return;
    return fazer(x.id, async () =>
      noTauri ? (await como<{ message: string }>(commands.cleanShaderCache(x.id))).message : `Demonstração: ${x.formatted} liberados.`,
    );
  };
</script>

<Cartao titulo="Configuração do jogo" apoio={jogo.configEmOutroLugar}>
  {#snippet selo()}
    {#if config?.aplicado}<span class="selo" data-tom="ok">perfil leve aplicado</span>{/if}
  {/snippet}
  {#if jogo.configEmOutroLugar}
    {@render configExtra?.()}
  {:else if lendo}
    <Esqueleto linhas={2} rotulo="Lendo a configuração" />
  {:else if configFalhou || !config}
    <Estado tipo="erro" compacto titulo={`Não consegui ler a configuração do ${jogo.nome}`} fazer="Feche o jogo e abra esta página de novo: com ele aberto o arquivo fica travado." />
  {:else if config.ilegivel}
    <p class="aviso">{config.ilegivel}</p>
  {:else if !config.arquivo}
    <p class="apoio">Não achei o arquivo de configuração. Abra o {jogo.nome} uma vez e feche: ele cria o arquivo.</p>
  {:else if config.previsao.length === 0}
    <p class="apoio">Já está no perfil leve do Otimiza: nada a mudar.</p>
  {:else}
    <p class="apoio">O perfil leve tira o que pesa na placa sem tirar o que mostra o inimigo. Só muda o que já existe no arquivo do jogo, e volta byte a byte.</p>
    <ul class="linhas">
      {#each config.previsao as p (p.chave)}
        <li>
          <div>
            <span><b>{p.chave}</b> {p.antes} → {p.depois}</span>
            <span class="apoio pequeno">{p.por_que}</span>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
  {#if recado && recadoEm === "config"}<p class="recado" role="status">{recado}</p>{/if}
  {#if config?.aberto}<p class="aviso">Feche o {jogo.nome} antes: ele reescreve o arquivo ao sair e apagaria a mudança.</p>{/if}
  {#snippet acoes()}
    {#if config && config.previsao.length > 0}
      <button class="btn primario" disabled={ocupado !== null || config.aberto} onclick={aplicar}>{ocupado === "config" ? "Gravando…" : "Aplicar perfil leve"}</button>
    {/if}
    {#if config?.aplicado}
      <button class="btn" disabled={ocupado !== null || config.aberto} onclick={() => config && desfazer(config)}>Desfazer</button>
    {/if}
  {/snippet}
</Cartao>

<Cartao titulo={cacheExtra ? "Cache" : "Cache de shader"} apoio="Cache velho, de antes de atualizar o driver ou o jogo, faz o jogo recompilar no meio da partida: são as travadinhas que aparecem do nada.">
  {#if lendo}
    <Esqueleto linhas={2} rotulo="Medindo os caches" />
  {:else if cacheFalhou}
    <Estado tipo="erro" compacto titulo="Não consegui medir os caches de shader" fazer="Abra esta página de novo em alguns segundos. Se repetir, abra o Otimiza como administrador." />
  {:else if caches.length === 0}
    <p class="apoio">Nenhum cache de shader do driver ocupando espaço para este jogo.</p>
  {:else}
    {#if cacheExtra}<span class="rotulo-grupo">Shaders do driver de vídeo</span>{/if}
    <ul class="linhas">
      {#each caches as x (x.id)}
        <li>
          <div>
            <span>{x.name} · <b>{x.formatted}</b></span>
            {#if x.stale}<span class="apoio pequeno">Mais velho que o driver: vale limpar.</span>{/if}
          </div>
          <button class="btn" disabled={ocupado !== null || !!config?.aberto} onclick={() => limpar(x)}>{ocupado === x.id ? "Limpando…" : "Limpar"}</button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if recado && recadoEm === "cache"}<p class="recado" role="status">{recado}</p>{/if}
  {@render cacheExtra?.()}
</Cartao>

<Cartao titulo="Windows e anticheat">
  <p class="apoio">{jogo.anticheat}</p>
</Cartao>
