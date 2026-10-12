# Laboratório E1: a captura do Frame Boost hoje (DXGI Duplicação) contra a Windows.Graphics.Capture, no jogo de teste
# com o marcador de quadros. Não precisa de administrador. Abre o jogo numa janela pequena no canto do monitor.
#   powershell -ExecutionPolicy Bypass -File scripts\laboratorio\bancada-captura.ps1
param(
  [int[]]$Fps = @(60, 120),
  [string[]]$Metodos = @('dxgi', 'wgc'),
  [int]$Segundos = 10,
  [int]$Repeticoes = 2,
  [int]$X = 1280, [int]$Y = 0, [int]$Largura = 640, [int]$Altura = 360,
  [switch]$TelaCheia
)
$raiz = Join-Path $PSScriptRoot '..\..\src-tauri\target\release\examples'
$jogo = Join-Path $raiz 'jogo_de_teste.exe'
$bancada = Join-Path $raiz 'bancada_captura.exe'
$pasta = Join-Path $env:TEMP 'otimiza-bancada-captura'
New-Item -ItemType Directory -Force $pasta | Out-Null

foreach ($f in $Fps) {
  for ($r = 1; $r -le $Repeticoes; $r++) {
    foreach ($m in $Metodos) {
      $marcador = Join-Path $pasta "marcador-$f-$m-$r.txt"
      $env:OTIMIZA_TESTE_X = "$X"; $env:OTIMIZA_TESTE_Y = "$Y"; $env:OTIMIZA_TESTE_COPIA = '1'
      $env:OTIMIZA_TESTE_MARCADOR = $marcador
      if ($TelaCheia) { $env:OTIMIZA_TESTE_TELA_CHEIA = '1' } else { Remove-Item Env:OTIMIZA_TESTE_TELA_CHEIA -ErrorAction SilentlyContinue }
      $p = Start-Process -FilePath $jogo -ArgumentList @("$f", "$Largura", "$Altura", '0') -PassThru -RedirectStandardOutput (Join-Path $pasta "jogo-$f-$m-$r.txt")
      Start-Sleep -Seconds 2
      $linha = & $bancada $m $Segundos $marcador 2>&1 | Select-Object -Last 1
      Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
      Start-Sleep -Milliseconds 700
      "jogo a $f fps, rodada $r · $linha"
    }
  }
}
Remove-Item Env:OTIMIZA_TESTE_MARCADOR, Env:OTIMIZA_TESTE_COPIA -ErrorAction SilentlyContinue
