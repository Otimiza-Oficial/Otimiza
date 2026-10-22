# Counter-Strike 2 — ficha de configuração

| | |
|---|---|
| Arquivo | `<Steam>\userdata\<conta>\730\local\cfg\cs2_video.txt` (um por conta; o Otimiza usa o mais recente) |
| Formato | KeyValues da Valve (`"chave"  "valor"`) |
| Perfil no código | `modules/windows/configmotor.rs`, id `cs2` |
| Situação | **escreve só o que já existe no arquivo**; os nomes das chaves estão **a confirmar** com o REA |

## O que o perfil muda
| Chave | Para | Por quê |
|---|---|---|
| `setting.videocfg_shadow_quality` | 0 | qualidade das sombras no mínimo |
| `setting.videocfg_ao_detail` | 0 | oclusão de ambiente: cara e não ajuda a ver ninguém |
| `setting.videocfg_particle_detail` | 0 | partículas pesam em granada e fumaça |

**Não toca** nas sombras dinâmicas dos jogadores: elas entregam posição (há teste que garante).

## O que confirmar com o REA
- os nomes exatos das chaves no `cs2_video.txt` da versão atual (chave com nome errado não escreve nada, mas também não ajuda);
- se o jogo relê o arquivo ao abrir ou só a configuração da nuvem da Steam;
- o ganho medido, antes × depois.
