# Fortnite — ficha de configuração

| | |
|---|---|
| Arquivo | `%LOCALAPPDATA%\FortniteGame\Saved\Config\WindowsClient\GameUserSettings.ini` |
| Formato | INI da Unreal (`[ScalabilityGroups]`, chaves `sg.*`, valores 0 a 3 ou 4) |
| Perfil no código | `modules/windows/configmotor.rs`, id `fortnite` |
| Situação | **escreve** (as chaves `sg.*` são o padrão de escalabilidade da Unreal) |

## O que o perfil muda
| Chave | Para | Por quê |
|---|---|---|
| `sg.ShadowQuality` | 0 | sombras: das opções mais caras na placa |
| `sg.PostProcessQuality` | 0 | desfoque e brilho: custam e atrapalham a leitura |
| `sg.EffectsQuality` | 0 | explosões e fumaça: pesam na hora da luta |
| `sg.FoliageQuality` | 0 | menos grama: menos custo e nada escondendo inimigo |

Não mexe em `sg.ViewDistanceQuality` (ver longe é jogo) nem em `sg.ResolutionQuality` (a escala é do Frame Boost).

## O que confirmar com o REA (bancada, nunca no cliente)
- se o jogo reescreve o arquivo inteiro ao fechar (o Otimiza já recusa com ele aberto);
- se o modo de renderização "Desempenho" usa outras chaves (`[PerformanceMode]`);
- o ganho medido em placa de entrada, antes × depois, no laboratório.
