# Laboratório E2: o tranco de cada jeito de entregar quadros (laboratorio/entrega-cpp/apresentador.exe), medido pela
# bancada de captura com a hora em que cada quadro foi para a tela. Revezando, com a MEDIANA das rodadas.
# Não precisa de administrador. Evite mexer no canto de cima, à direita, do monitor principal enquanto roda.
param([int]$Repeticoes = 3, [int]$Segundos = 8)
$base = Join-Path $PSScriptRoot '..\..'
$apr = Join-Path $base 'laboratorio\entrega-cpp\apresentador.exe'
$banc = Join-Path $base 'src-tauri\target\release\examples\bancada_captura.exe'
$casos = @(
  @{ Modo = 'cronometro'; Fps = 120 }, @{ Modo = 'horario'; Fps = 120 },
  @{ Modo = 'monitor'; Fps = 180 }, @{ Modo = 'cronometro'; Fps = 180 }, @{ Modo = 'horario'; Fps = 180 }
)
$env:OTIMIZA_TESTE_X = '1280'; $env:OTIMIZA_TESTE_Y = '0'; $env:BANCADA_HORA = 'tela'
Remove-Item Env:BANCADA_DEPURAR -ErrorAction SilentlyContinue
$pasta = Join-Path $env:TEMP 'otimiza-bancada-entrega'; New-Item -ItemType Directory -Force $pasta | Out-Null
$dados = @()
for ($r = 1; $r -le $Repeticoes; $r++) {
  foreach ($c in $casos) {
    Get-Process apresentador, jogo_de_teste -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500
    $m = Join-Path $pasta "$($c.Modo)-$($c.Fps)-$r.txt"
    $p = Start-Process -FilePath $apr -ArgumentList @($c.Modo, "$($c.Fps)", "$($Segundos + 4)", $m) -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 2
    $linha = "$(& $banc dxgi $Segundos $m 2>&1 | Select-Object -Last 1)"
    Get-Process -Id $p.Id -ErrorAction SilentlyContinue | Stop-Process -Force
    "{0,-10} {1,3} r{2} | {3}" -f $c.Modo, $c.Fps, $r, ($linha -replace '^dxgi \| ', '')
    if ($linha -match 'jogo ([\d.]+) fps.*?perdidos \d+ \(([\d.]+)%\).*?p99 ([\d.]+) ms max ([\d.]+) ms \| atraso p50 ([\d.]+) ms.*?tranco ([\d.]+)%' -and [double]$matches[1] -gt 0) {
      $dados += [pscustomobject]@{ Caso = "$($c.Modo) $($c.Fps)"; Perdidos = [double]$matches[2]; P99 = [double]$matches[3]; Max = [double]$matches[4]; Atraso = [double]$matches[5]; Tranco = [double]$matches[6] }
    }
  }
}
function Mediana($v) { $s = @($v | Sort-Object); if ($s.Count -eq 0) { return [double]::NaN }; $k = [int][Math]::Floor($s.Count / 2); if ($s.Count % 2) { $s[$k] } else { ($s[$k - 1] + $s[$k]) / 2 } }
''
"MEDIANAS ({0} rodadas; rodada inválida, com o canto coberto, fica de fora)" -f $Repeticoes
'{0,-15} | {1,7} | {2,9} | {3,9} | {4,9} | {5,6}' -f 'caso', 'tranco', 'perdidos', 'p99', 'pior', 'atraso'
foreach ($g in ($dados | Group-Object Caso)) {
  $v = $g.Group
  '{0,-15} | {1,6:N1}% | {2,8:N1}% | {3,6:N1} ms | {4,6:N1} ms | {5,4:N1} ms ({6} válidas)' -f $g.Name, (Mediana $v.Tranco), (Mediana $v.Perdidos), (Mediana $v.P99), (Mediana $v.Max), (Mediana $v.Atraso), $v.Count
}
