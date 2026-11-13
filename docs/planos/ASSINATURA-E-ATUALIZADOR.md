# Assinatura de código e atualizador — pesquisa de 05/11/2026

> Pesquisa feita para a Onda 0 da Nova Geração (`PLANO-1.0-NOVA-ERA.md`). Corrige pontos de `docs/app/ASSINATURA.md`.
> O que não pôde ser confirmado em fonte primária está marcado **não verificado**.

## Resumo

- **Azure Artifact Signing (o antigo Trusted Signing) não aceita o Brasil**, nem pessoa física nem empresa.
- **Recomendação:** certificado IV (pessoa física) ou OV (empresa) na **SSL.com, guardado no eSigner** (nuvem), que assina direto no GitHub Actions.
- **Nenhum certificado, nem o EV, tira o aviso do SmartScreen no primeiro dia** desde 2024. Assinado, o aviso passa a mostrar o nome do editor e a reputação se acumula entre versões.

## 0. Fatos que mudam o plano

### Artifact Signing
- Empresas aceitas: EUA, Canadá, UE, Reino Unido, Austrália, Nova Zelândia, Japão, Coreia do Sul, Singapura, Suíça, Noruega, Israel.
- Pessoas físicas: só EUA e Canadá.
- **O Brasil não está na lista.** A região "Brazil South" do Azure é só onde a conta fica hospedada; não muda a elegibilidade.
- Também não dá confiança imediata no SmartScreen ("does not provide instant SmartScreen trust").
- Fontes: https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart · https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options

### EV
- O EV não pula mais o SmartScreen desde 2024. Texto da Microsoft: *"EV certificates no longer bypass SmartScreen… Paying a premium for EV solely to avoid SmartScreen warnings is no longer justified."*
- Fonte: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation
- **Corrige o `ASSINATURA.md`**, que dizia "EV = reputação imediata".

### Validade do certificado
- Máximo de **460 dias** para certificados emitidos a partir de 01/03/2026 (Ballot CSC-31).
- Certum aplica 459 dias; SSL.com aplica 458.
- O texto do ballot não foi lido na fonte: **não verificado**.
- Na prática, os produtos de "2–3 anos" vêm com reemissões. Renovar a cada ~15 meses, sempre com o mesmo nome de editor.

### Chave em hardware
- Desde junho de 2023 a chave precisa ficar em hardware ou em HSM na nuvem. Não existe mais `.pfx` em arquivo.
- O exemplo de importar `.pfx` da documentação do Tauri não serve para certificado novo.

### Como a reputação se forma
- Pela assinatura do editor e pelo hash do arquivo, conforme o volume de instalações limpas ("several weeks and hundreds of clean installs").
- Não existe envio manual para ganhar reputação.
- O portal https://www.microsoft.com/en-us/wdsi/filesubmission serve para falso positivo do Defender.
- O Smart App Control do Windows 11 bloqueia arquivo sem assinatura e sem reputação.

### Risco específico de um otimizador
- Os critérios de software indesejado da Microsoft incluem *"Display exaggerated claims about your device's health"*.
- Uma reputação negativa contamina o certificado inteiro. Revisar os textos do app e do site.
- Fonte: https://learn.microsoft.com/en-us/unified-secops/criteria

### PresentMon
- O PresentMon embutido já vem assinado pela Intel (conferido localmente).
- O bundler do Tauri não assina de novo sidecar que já tem assinatura válida (`sign_binaries_if_needed`).

## (a) Opções

| Opção | Custo/ano | Brasil PF / PJ | SmartScreen | GitHub Actions | Prazo | Fonte |
|---|---|---|---|---|---|---|
| Azure Artifact Signing | ~US$ 120 | **Não / Não** | Acumula; não é instantâneo | Sim | — | quickstart, options |
| **SSL.com IV + eSigner** (PF) | US$ 129 do certificado + eSigner a partir de US$ 180/ano (240 assinaturas). Se o eSigner já inclui o certificado: **não verificado** | Sim (documento com foto e selfie). Aceita RG/CNH? **não verificado** | Como OV: acumula, mostra o nome | **Sim** (`SSLcom/esigner-codesign`, TOTP) | 3–5 dias | ssl.com/products/software-integrity/code-signing/iv/ |
| **SSL.com OV + eSigner** (PJ/MEI) | OV **não verificado** (faixa US$ 150–300) + eSigner | PJ em tese; CNPJ **não verificado** | Igual | Sim | **não verificado** | idem |
| Certum Standard em nuvem (SimplySign) | a partir de €209 | Pessoa física e organização | Igual | **Difícil**: código do celular a cada sessão | **não verificado** | shop.certum.eu/code-signing.html |
| Certum com cartão | a partir de €169 | Idem | Igual | **Não**: só em máquina local | Envio físico | idem |
| Certum Open Source | €49 | — | — | — | — | **Não serve**: o Otimiza é pago |
| DigiCert + KeyLocker | ~US$ 400+ (**não verificado**), 1.000 assinaturas/certificado | Brasil **não verificado** | Igual | Sim | **não verificado** | docs.digicert.com |
| EV | US$ 400+ | PJ | **Igual ao OV desde 2024** | Depende do provedor | Mais longo | options |
| Microsoft Store (MSIX) | Grátis | Mundial | Sem aviso | Pipeline separado | Revisão | — |

**Sobre a Microsoft Store:** o Tauri não gera MSIX. Não foi verificado se um app que mexe em HKLM e em serviços cabe no MSIX, nem se a Store aceita licença vendida por fora.

**Sobre ICP-Brasil:** e-CPF e e-CNPJ não são certificados Authenticode de assinatura de código (**não verificado**).

## (b) Caminho recomendado

**SSL.com eSigner: IV se o titular for pessoa física, OV se houver CNPJ.** É a única opção para o Brasil que se verificou rodando sozinha no GitHub Actions. A alternativa mais barata é a Certum em nuvem com assinatura manual.

**Atenção para MEI:** a razão social costuma conter o CPF, e o nome do certificado é o nome legal. Perguntar à SSL.com antes de comprar.

### O que o dono faz
1. **Decidir o titular:** CPF (IV) ou CNPJ (OV). O nome escolhido aparece no aviso e não deve mudar depois.
2. **Perguntar à SSL.com, por escrito:**
   - se aceitam RG/CNH e CNPJ;
   - se o eSigner é cobrado à parte;
   - como fica a razão social de MEI.
3. **Comprar o certificado com eSigner.** O prazo de 2–3 anos já vem com reemissões. Contratar o eSigner nível 1 anual.
4. **Validar a identidade.**
5. **Ativar o eSigner com TOTP e guardar o segredo** num gerenciador de senhas. Passar à engenharia: usuário, senha, credential_id e segredo TOTP.
6. **Guardar o backup offline da chave do atualizador** (seção d).
7. **Manter o aviso aos clientes** ("Mais informações → Executar assim mesmo") até a reputação se formar.
8. **Se o Defender acusar o app,** enviar pelo portal como "software developer".

### O que a engenharia faz
1. **Environment `release` no GitHub**, restrito a tags `v*`, com aprovação. Segredos: `ES_USERNAME`, `ES_PASSWORD`, `ES_CREDENTIAL_ID`, `ES_TOTP_SECRET`, `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
2. **Configuração de assinatura só para o CI** (`tauri.ci.conf.json` + `scripts/assinar.ps1`, seção c). O build local continua sem segredos.
3. **Considerar tirar o alvo `msi`:** gasta assinaturas e pode levar a instalação duplicada pelo atualizador.
4. **Passo no workflow que falha** se o `Get-AuthenticodeSignature` não for `Valid`.
5. **Corrigir o `docs/app/ASSINATURA.md`.**

## (c) Configuração proposta (não aplicada)

O Tauri assina o exe, os sidecars ainda sem assinatura, as DLLs de plugin do NSIS, o desinstalador e o instalador. São ~8–12 assinaturas por release com NSIS+MSI (**não verificado**); 240 por ano dão ~20 releases.

`pc-optimizer/src-tauri/tauri.ci.conf.json`:
```json
{
  "bundle": {
    "windows": {
      "digestAlgorithm": "sha256",
      "signCommand": "powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/assinar.ps1 %1"
    }
  }
}
```
O diretório em que o `signCommand` roda: **não verificado**. Se o caminho relativo falhar, usar caminho absoluto.

`pc-optimizer/src-tauri/scripts/assinar.ps1`:
```powershell
param([Parameter(Mandatory)][string]$Arquivo)
$ErrorActionPreference = 'Stop'
& "$env:CODESIGNTOOL_DIR\CodeSignTool.bat" sign `
  -username="$env:ES_USERNAME" -password="$env:ES_PASSWORD" `
  -credential_id="$env:ES_CREDENTIAL_ID" -totp_secret="$env:ES_TOTP_SECRET" `
  -input_file_path="$Arquivo" -override=true
if ($LASTEXITCODE -ne 0) { throw "Falha ao assinar $Arquivo" }
```
Parâmetros do CodeSignTool e servidor de carimbo de tempo (`http://ts.ssl.com`): **não verificados**. Conferir no README de https://github.com/SSLcom/esigner-codesign.

Trechos do `release.yml`:
```yaml
  compilar:
    environment: release
    ...
      - name: Instalar CodeSignTool
        shell: pwsh
        run: |
          Invoke-WebRequest "<URL_OFICIAL_CODESIGNTOOL_WINDOWS>" -OutFile cst.zip   # URL: não verificada
          Expand-Archive cst.zip -DestinationPath "$env:RUNNER_TEMP\cst"
          "CODESIGNTOOL_DIR=$env:RUNNER_TEMP\cst" | Out-File $env:GITHUB_ENV -Append -Encoding utf8

      - name: Compilar e publicar
        uses: tauri-apps/tauri-action@v0     # avaliar @v1
        env:
          ES_USERNAME: ${{ secrets.ES_USERNAME }}
          ES_PASSWORD: ${{ secrets.ES_PASSWORD }}
          ES_CREDENTIAL_ID: ${{ secrets.ES_CREDENTIAL_ID }}
          ES_TOTP_SECRET: ${{ secrets.ES_TOTP_SECRET }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          projectPath: pc-optimizer
          args: --config src-tauri/tauri.ci.conf.json
          updaterJsonPreferNsis: true        # o padrão é false: o latest.json apontaria para o MSI

      - name: Conferir assinatura
        shell: pwsh
        run: |
          $alvos = @(Get-ChildItem pc-optimizer/src-tauri/target/release/bundle/nsis/*-setup.exe) +
                   @(Get-Item pc-optimizer/src-tauri/target/release/pc-optimizer.exe)
          foreach ($a in $alvos) {
            $s = Get-AuthenticodeSignature $a.FullName
            if ($s.Status -ne 'Valid') { throw "$($a.Name): $($s.Status)" }
          }
```

## (d) Atualizador: da 3.6 (só faixa de aviso) para a 4.0 (plugin)

### O plugin
- `tauri-plugin-updater` **2.14.1** (28/10/2026), Rust 1.90 ou mais novo.
- Só instala versão maior: 4.0.0 > 3.6.0. Downgrade só com um `version_comparator` próprio; **não usar**.
- O endpoint `releases/latest/` ignora pré-releases.

### Passos
1. **Chaves:** `npm run tauri signer generate -- -w ~/.tauri/otimiza-updater.key`, com senha.
   - Vão para os segredos do GitHub.
   - Backup offline com o dono em dois lugares. **Perder a chave significa não conseguir atualizar mais ninguém.**
2. **`tauri.conf.json`:**
   - `bundle.createUpdaterArtifacts: true` (não `"v1Compatible"`).
   - `plugins.updater` com `pubkey`, endpoint `https://github.com/Otimiza-Oficial/Otimiza/releases/latest/download/latest.json` e `windows.installMode: "passive"`.
   - **Manter `identifier: com.otimiza.console`.**
3. **Código:**
   - `tauri-plugin-updater` em `lib.rs` (`#[cfg(desktop)]`).
   - `updater:default` nas capabilities.
   - Fluxo pelo Rust (`check()` → `download_and_install`), mostrando a faixa com o botão "Atualizar agora".
4. **CI:** os segredos de (c) e `updaterJsonPreferNsis: true`.
   - A assinatura Authenticode vem antes do `.sig`.
   - **Nunca reassinar o instalador depois.**
5. **Clientes 3.x:** só são alcançados pela faixa de aviso.
   - O teste `a_nova_geracao_e_atualizacao_para_quem_esta_na_3` garante que eles veem a 4.0.
   - Divulgar também no Discord.
   - A primeira atualização automática de verdade é 4.0.0 → 4.0.1.
6. **Plano B na 4.x:** se o `check()` falhar, cair na faixa antiga com o link da página.
7. **Teste antes da 4.0 pública:**
   - rc com `latest.json` de teste;
   - download, assinatura, instalação passiva e reinício;
   - **`.sig` adulterado tem que ser rejeitado.**
8. **Troca de chave:** publicar uma versão assinada com a chave antiga trazendo a `pubkey` nova. Ativar 2FA na conta do GitHub e exigir aprovação no Environment.

## (e) Riscos

- **O aviso continua nas primeiras semanas,** mesmo assinado. Comunicar isso aos clientes.
- **Textos alarmistas sobre a "saúde do PC"** levam a reputação negativa, que contamina o certificado.
- **NSIS + MSI juntos:** o atualizador pode apontar para o MSI e deixar duas instalações. Usar `updaterJsonPreferNsis` ou tirar o MSI.
- **Renovação a cada ~15 meses.** Sem carimbo de tempo, as versões antigas viram inválidas quando o certificado vence.
- **Cota do eSigner:** assinar só em tags.
- **TOTP + senha do eSigner valem tanto quanto a chave.** Restringir ao Environment; nunca disponível em PR de fork.
- **Perda da chave do atualizador** prende os clientes 4.x; a saída é o plano B (d.6).
- **Ainda não verificado:**
  - se a SSL.com aceita documentos brasileiros e CNPJ;
  - se o eSigner é cobrado à parte;
  - preço do OV e da DigiCert;
  - parâmetros do CodeSignTool;
  - diretório do `signCommand`;
  - diferenças entre o tauri-action v0 e v1.
