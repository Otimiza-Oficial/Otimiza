# Valorant — ficha de configuração

| | |
|---|---|
| Arquivo | `%LOCALAPPDATA%\VALORANT\Saved\Config\<conta>\Windows\GameUserSettings.ini` (um por conta Riot; o Otimiza usa o mais recente) |
| Formato | INI da Unreal, seção `[ScalabilityGroups]` |
| Perfil no código | `modules/windows/configmotor.rs`, id `valorant` |
| Situação | **escreve só o que já existe no arquivo, com o jogo fechado** (1.0.3) |

## O que o perfil muda
| Chave | Para | Por quê |
|---|---|---|
| `sg.ShadowQuality` | 0 | sombras: das opções mais caras |
| `sg.PostProcessQuality` | 0 | brilho e distorção: custam e embaçam |
| `sg.EffectsQuality` | 0 | efeitos das habilidades: pesam na troca de tiro |

**Não toca** em textura, serrilhado nem resolução (há teste que garante): mexem na leitura da tela.

## Vanguard
- O Otimiza só escreve o arquivo de configuração que o próprio jogo grava, com o jogo fechado: o mesmo que mudar no menu.
- Nunca abre o processo do jogo, nunca desenha por cima dele: o Frame Boost e a mira ficam recusados com o Valorant aberto.

## O que confirmar
- se a Riot devolve a configuração da nuvem ao entrar: se devolver, o painel volta a mostrar o que mudaria (a prévia lê o arquivo de novo);
- o ganho medido, antes × depois.
