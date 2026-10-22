# Apex Legends — ficha de configuração

| | |
|---|---|
| Arquivo | `%USERPROFILE%\Saved Games\Respawn\Apex\local\videoconfig.txt` |
| Formato | KeyValues (`"setting.chave"  "valor"`) |
| Perfil no código | `modules/windows/configmotor.rs`, id `apex` |
| Situação | **escreve só o que já existe no arquivo**; os nomes das chaves estão **a confirmar** com o REA |

## O que o perfil muda
| Chave | Para | Por quê |
|---|---|---|
| `setting.ssao_enabled` | 0 | oclusão de ambiente: cara e quase invisível em movimento |
| `setting.volumetric_lighting` | 0 | luz volumétrica: cara e às vezes esconde inimigo na contraluz |

## O que confirmar com o REA
- os nomes exatos das chaves na versão atual;
- se o EAC se importa com o arquivo ser alterado com o jogo fechado (é a configuração que o próprio jogo grava; o Otimiza nunca toca o executável);
- o ganho medido, antes × depois.
