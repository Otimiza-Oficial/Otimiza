<script lang="ts">
  // O perfil gráfico do GTA/FiveM pelo Turbo (trazido da interface 3.3/3.6, `perfilDoJogoNoTurbo`). O jogo reescreve a
  // configuração ao sair, então o pedido espera ele fechar e o Otimiza aplica sozinho, com o arquivo de antes no
  // histórico. Só aparece quando o Rust oferece o perfil para o jogo aberto (`turbo_perfil_do_jogo`).
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Perfil = { oferece: boolean; pendente: string | null; aplicado: string | null };

  const NOME: Record<string, string> = { competitivo: "Máximo de FPS", equilibrado: "Equilibrado", sem_teto: "Sem limite de quadros" };
  const nome = (id: string) => NOME[id] ?? id;

  /** O jogo aberto, vindo do Turbo: trocar de jogo relê. */
  let { jogo = null }: { jogo?: string | null } = $props();

  let p = $state<Perfil | null>(null);
  let ocupado = $state(false);
  let erro = $state<string | null>(null);

  $effect(() => {
    void jogo;
    if (!noTauri) {
      p = { oferece: true, pendente: null, aplicado: null };
      return;
    }
    invoke<Perfil>("turbo_perfil_do_jogo")
      .then((r) => (p = r))
      .catch(() => (p = null));
  });

  async function pedir(perfil: string | null) {
    ocupado = true;
    erro = null;
    try {
      p = noTauri ? await invoke<Perfil>("turbo_perfil_do_jogo_pedir", { perfil }) : { oferece: true, pendente: perfil, aplicado: null };
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = false;
    }
  }
</script>

{#if p?.oferece}
  <section class="secao">
    <div class="linha-perfil">
      <div>
        <h2>Perfil gráfico do jogo</h2>
        {#if p.pendente}
          <p class="apoio">
            {nome(p.pendente)}, pedido. O Otimiza aplica quando você fechar o jogo, e desfazer devolve o arquivo exatamente
            como estava.
          </p>
        {:else if p.aplicado}
          <p class="apoio">{nome(p.aplicado)}, aplicado pelo Otimiza. Desfaz no Histórico.</p>
        {:else}
          <p class="apoio">
            O que mais pesa no FiveM e no GTA: suavização (MSAA), sombras, reflexos e grama. O jogo reescreve a
            configuração ao sair, então o Otimiza aplica quando você fechar o jogo.
          </p>
        {/if}
      </div>
      {#if p.pendente}
        <button class="btn" disabled={ocupado} onclick={() => pedir(null)}>Cancelar o pedido</button>
      {:else if !p.aplicado}
        <div class="acoes">
          <button class="btn" disabled={ocupado} onclick={() => pedir("equilibrado")}>Equilibrado</button>
          <button class="btn" disabled={ocupado} onclick={() => pedir("competitivo")}>Máximo de FPS</button>
        </div>
      {/if}
    </div>
    {#if erro}<p class="erro">{erro}</p>{/if}
  </section>
{/if}

<style>
  .linha-perfil {
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
  }
  .linha-perfil > div:first-child {
    max-width: 60ch;
  }
  .acoes {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  p {
    margin: 0;
  }
  .erro {
    margin-top: 8px;
    color: var(--perigo);
    font-size: 13px;
  }
</style>
