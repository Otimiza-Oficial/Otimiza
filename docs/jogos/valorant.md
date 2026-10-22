# Valorant — ficha de configuração

| | |
|---|---|
| Arquivo | `%LOCALAPPDATA%\VALORANT\Saved\Config\<conta>\Windows\GameUserSettings.ini` |
| Formato | INI da Unreal |
| Perfil no código | **nenhum: o Otimiza não escreve** |
| Situação | **em estudo** |

## Por que ainda não escreve
- a Riot sincroniza a configuração pela conta: o arquivo local pode ser sobrescrito ao entrar, e a mudança sumiria sem aviso;
- o Vanguard é de núcleo; mesmo sendo só o arquivo de configuração, nada entra sem confirmar que é seguro.

## O que confirmar com o REA
- onde a configuração gráfica realmente mora (arquivo local, `RiotUserSettings.ini` ou nuvem) e qual vence;
- se mudar com o jogo fechado vale ao abrir.
