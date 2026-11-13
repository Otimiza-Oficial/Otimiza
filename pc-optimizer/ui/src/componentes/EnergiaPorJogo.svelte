<script lang="ts">
  import { como } from "../lib/contrato";
  // Energia por jogo (trazido da interface antiga, `src/energia.ts`): o vencedor da calibração guardado para o jogo
  // aberto, a lista dos jogos com perfil e o modo dinâmico — ligado, o Otimiza troca para o perfil do jogo quando ele
  // abre e volta ao plano de antes quando ele fecha.
  import { onMount } from "svelte";
    import { commands } from "../lib/tipos";
  import { noTauri } from "../lib/ponte";

  type Painel = { perfis_de_jogo: { executavel: string; quando: number }[]; dinamico: boolean };

  let perfis = $state<string[]>([]);
  let dinamico = $state(false);
  let lido = $state(false);
  let ocupado = $state(false);
  let recado = $state<string | null>(null);

  async function carregar() {
    if (!noTauri) {
      lido = true;
      return;
    }
    try {
      const p = await como<Painel>(commands.energiaPainel());
      perfis = p.perfis_de_jogo.map((x) => x.executavel);
      dinamico = p.dinamico;
    } catch (e) {
      recado = String(e);
    }
    lido = true;
  }

  onMount(carregar);

  async function agir(f: () => Promise<unknown>) {
    ocupado = true;
    recado = null;
    try {
      await f();
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = false;
    }
  }

  async function guardar() {
    await agir(async () => {
      if (!noTauri) {
        perfis = [...new Set([...perfis, "FiveM_GTAProcess.exe"])];
        recado = "Guardado para FiveM_GTAProcess.exe.";
        return;
      }
      const j = await como<{ executavel: string } | null>(commands.jogoAberto());
      if (!j) throw "Abra o jogo para guardar o perfil dele.";
      await commands.energiaSalvarCalibradoParaOJogo(j.executavel);
      recado = `Guardado para ${j.executavel}.${dinamico ? "" : " Ligue o modo dinâmico para ele entrar sozinho."}`;
    });
  }
</script>

<section class="secao">
  <h2>Energia por jogo</h2>
  <p class="apoio">Calibrou com o jogo aberto? Guarde o vencedor para esse jogo. Com o modo dinâmico ligado, o Otimiza troca para ele quando o jogo abre e volta ao seu plano quando fecha.</p>
  <div class="acoes">
    <button class="btn" disabled={ocupado} onclick={guardar}>Guardar o vencedor para o jogo aberto</button>
  </div>
  {#if lido}
    <div class="linha">
      <div>
        <span>Modo dinâmico</span>
        <p class="apoio pequeno">{perfis.length ? `${perfis.length} ${perfis.length === 1 ? "jogo" : "jogos"} com perfil.` : "Nenhum jogo com perfil ainda."}</p>
      </div>
      <button class="interruptor" role="switch" aria-checked={dinamico} aria-label="Modo dinâmico" disabled={ocupado}
        onclick={() => agir(async () => {
          if (noTauri) await commands.energiaModoDinamico(!dinamico);
          else dinamico = !dinamico;
        })}></button>
    </div>
    {#each perfis as exe (exe)}
      <div class="linha">
        <span>{exe}</span>
        <button class="btn quieto" disabled={ocupado} onclick={() => agir(async () => {
          if (noTauri) await commands.energiaRemoverPerfilDeJogo(exe);
          else perfis = perfis.filter((x) => x !== exe);
        })}>Tirar</button>
      </div>
    {/each}
  {/if}
  {#if recado}<p class="recado">{recado}</p>{/if}
</section>

<style>
  .secao {
    display: grid;
    gap: 10px;
  }
  p {
    margin: 0;
  }
  .acoes {
    display: flex;
    gap: 10px;
  }
  .linha {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 13px;
  }
  .pequeno {
    font-size: 12px;
  }
  .recado {
    font-size: 13px;
  }
</style>
