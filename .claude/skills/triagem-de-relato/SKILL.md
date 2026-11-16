---
name: triagem-de-relato
description: "Use quando o dono trouxer uma reclamação de cliente ou um print ('tá pesado', 'nenhum gerador funciona', 'piorou', 'não liga'): leva do relato à causa com prova, e devolve o que pedir ao cliente e a mensagem pronta."
---

# Triagem de relato

**Regra:** a causa sai da prova (log, print, código), nunca do último caso parecido.

1. **Print:** procure o texto exato no código (`git grep "Nenhum gerador funciona aqui"`) e siga o caminho até a
   condição que o mostra. Ex. 16/11: a mensagem vinha do anticheat do Valorant que roda desde o boot.
2. **Peça o que falta:** log (Ajustes → Registro do Otimiza → Salvar em Downloads), jogo, Hz e G-Sync/FreeSync,
   placa, versão, o que estava ligado (Turbo, Frame Boost, Jogo liso, botão "Jogar").
3. **Com log:** skill `ler-log-do-cliente` / agente `analista-de-log`.
4. **Responda ao dono** com: causa provada ou hipótese marcada como tal, o que o cliente faz agora (ex.: desfazer
   X no Histórico), e se precisa de correção numa versão.
Agente: `triagem-de-suporte`.
