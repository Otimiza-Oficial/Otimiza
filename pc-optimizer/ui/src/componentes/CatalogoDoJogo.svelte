<script lang="ts">
  // O catálogo de um jogo no destaque da aba Jogos (a lista em `lib/catalogo.ts`): a configuração do próprio jogo (o
  // perfil de `configmotor.rs`, só chaves que já existem, com o jogo fechado e desfazer byte a byte), os caches de
  // shader que este jogo usa (os do driver e, quando há, o da Steam) e o que o anticheat deixa o Otimiza fazer.
  import { onMount } from "svelte";
  import type { JogoDoCatalogo } from "../lib/catalogo";
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import { noTauri } from "../lib/ponte";

  let { jogo }: { jogo: JogoDoCatalogo } = $props();

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
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);

  async function carregar() {
    if (!noTauri) {
      ({ config, caches } = demo());
      return;
    }
    const [c, s] = await Promise.allSettled([
      jogo.perfil ? como<Config[]>(commands.configMotorPainel()) : Promise.resolve([] as Config[]),
      como<{ caches: Cache[] }>(commands.analyzeShaders()),
    ]);
    config = c.status === "fulfilled" ? (c.value.find((x) => x.id === jogo.perfil) ?? null) : null;
    caches = s.status === "fulfilled" ? s.value.caches.filter((x) => jogo.caches.includes(x.id) && x.bytes > 0) : [];
  }

  onMount(carregar);

  async function fazer(chave: string, acao: () => Promise<string>) {
    ocupado = chave;
    recado = null;
    try {
      recado = await acao();
      await carregar();
    } catch (e) {
      recado = String(e);
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

  const limpar = (x: Cache) => {
    if (!confirm(`Apagar ${x.name} (${x.formatted})? A primeira partida recompila os shaders e pode engasgar; da segunda em diante fica mais liso.`)) return;
    return fazer(x.id, async () =>
      noTauri ? (await como<{ message: string }>(commands.cleanShaderCache(x.id))).message : `Demonstração: ${x.formatted} liberados.`,
    );
  };
</script>

<div class="catalogo">
  <span class="titulo">Catálogo do {jogo.nome}</span>

  <div class="bloco">
    <span class="sub">Configuração do jogo</span>
    {#if jogo.configEmOutroLugar}
      <p class="apoio">{jogo.configEmOutroLugar}</p>
    {:else if !config}
      <p class="apoio">Não consegui ler a configuração deste jogo agora.</p>
    {:else if config.ilegivel}
      <p class="aviso">{config.ilegivel}</p>
    {:else if !config.arquivo}
      <p class="apoio">Não achei o arquivo de configuração. Abra o {jogo.nome} uma vez e feche: ele cria o arquivo.</p>
    {:else if config.previsao.length === 0}
      <p class="apoio">Já está no perfil leve do Otimiza: nada a mudar.</p>
    {:else}
      <p class="apoio">O perfil leve tira o que pesa na placa sem tirar o que mostra o inimigo. Só muda o que já existe no arquivo do jogo, e volta byte a byte.</p>
      <ul>
        {#each config.previsao as p (p.chave)}
          <li>
            <div>
              <span>{p.chave} · {p.antes} → {p.depois}</span>
              <p class="apoio pequeno">{p.por_que}</p>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
    {#if config?.aberto}<p class="aviso">Feche o {jogo.nome} antes: ele reescreve o arquivo ao sair e apagaria a mudança.</p>{/if}
    <div class="acoes">
      {#if config && config.previsao.length > 0}
        <button class="btn" disabled={ocupado !== null || config.aberto} onclick={aplicar}>{ocupado === "config" ? "Gravando…" : "Aplicar perfil leve"}</button>
      {/if}
      {#if config?.aplicado}
        <button class="btn" disabled={ocupado !== null || config.aberto} onclick={() => config && desfazer(config)}>Desfazer</button>
      {/if}
    </div>
  </div>

  <div class="bloco">
    <span class="sub">Cache de shader</span>
    {#if caches.length === 0}
      <p class="apoio">Nenhum cache de shader deste jogo ocupando espaço.</p>
    {:else}
      <p class="apoio">Cache velho, de antes de atualizar o driver ou o jogo, faz o jogo recompilar no meio da partida: são as travadinhas que aparecem do nada.</p>
      <ul>
        {#each caches as x (x.id)}
          <li>
            <div>
              <span>{x.name} · {x.formatted}</span>
              {#if x.stale}<p class="apoio pequeno">Mais velho que o driver: vale limpar.</p>{/if}
            </div>
            <button class="btn" disabled={ocupado !== null || !!config?.aberto} onclick={() => limpar(x)}>{ocupado === x.id ? "Limpando…" : "Limpar"}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="bloco">
    <span class="sub">Windows e anticheat</span>
    <p class="apoio">{jogo.anticheat}</p>
  </div>

  {#if recado}<p class="recado">{recado}</p>{/if}
</div>

<style>
  .catalogo {
    display: grid;
    gap: 10px;
    padding: 16px 0 0;
    border-top: 1px solid var(--linha);
  }
  .titulo {
    font-weight: 600;
  }
  .bloco {
    display: grid;
    gap: 6px;
  }
  .sub {
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  p {
    margin: 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .acoes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .pequeno {
    font-size: 12px;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .recado {
    font-size: 13px;
  }
</style>
