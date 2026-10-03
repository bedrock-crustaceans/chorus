param(
    [Parameter(Mandatory = $true)][string]$Server,
    [Parameter(Mandatory = $true)][long]$Seed,
    [Parameter(Mandatory = $true)][string]$Areas,
    [int]$SettleSeconds = 20
)

$ErrorActionPreference = 'Stop'
$level = "parity-$Seed"
$world = Join-Path $Server "worlds\$level"
if (Test-Path $world) { Remove-Item -Recurse -Force $world }

$properties = Join-Path $Server 'server.properties'
$overrides = @{
    'level-name'            = $level
    'level-seed'            = "$Seed"
    'server-port'           = '19152'
    'server-portv6'         = '19153'
    'enable-lan-visibility' = 'false'
    'online-mode'           = 'false'
    'allow-list'            = 'false'
    'tick-distance'         = '4'
}
$lines = Get-Content $properties | ForEach-Object {
    $key = ($_ -split '=', 2)[0]
    if ($overrides.ContainsKey($key)) { "$key=$($overrides[$key])" } else { $_ }
}
Set-Content -Path $properties -Value $lines -Encoding ascii

$start = New-Object System.Diagnostics.ProcessStartInfo
$start.FileName = Join-Path $Server 'bedrock_server.exe'
$start.WorkingDirectory = $Server
$start.UseShellExecute = $false
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$process = New-Object System.Diagnostics.Process
$process.StartInfo = $start
$output = New-Object System.Collections.Concurrent.ConcurrentQueue[string]
$subscription = Register-ObjectEvent -InputObject $process -EventName OutputDataReceived -MessageData $output -Action {
    if ($null -ne $EventArgs.Data) { $Event.MessageData.Enqueue($EventArgs.Data) }
}
$process.Start() | Out-Null
$process.BeginOutputReadLine()

function Drain([string]$pattern) {
    $line = $null
    while ($output.TryDequeue([ref]$line)) {
        Write-Host "  bds> $line"
        if ($pattern -and $line -match $pattern) { return $true }
    }
    return $false
}

function Wait-For([string]$pattern, [int]$seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        if (Drain $pattern) { return }
        if ($process.HasExited) { throw "server exited while waiting for '$pattern'" }
        Start-Sleep -Milliseconds 200
    }
    throw "timed out waiting for '$pattern'"
}

try {
    Wait-For 'Server started' 120
    # the first write carries a UTF-8 BOM, so spend it on an empty line
    $process.StandardInput.WriteLine('')
    $index = 0
    foreach ($area in $Areas -split ';') {
        $c = $area -split ',' | ForEach-Object { [int]$_ }
        $from = "$($c[0] * 16) 0 $($c[1] * 16)"
        $to = "$($c[2] * 16 + 15) 0 $($c[3] * 16 + 15)"
        $process.StandardInput.WriteLine("tickingarea add $from $to parity$index true")
        $index++
    }
    $deadline = (Get-Date).AddSeconds($SettleSeconds)
    while ((Get-Date) -lt $deadline) {
        Drain $null | Out-Null
        Start-Sleep -Milliseconds 200
    }
    # bds only saves modified chunks, so touch the always-empty top layer of each one
    foreach ($block in 'glass', 'air') {
        foreach ($area in $Areas -split ';') {
            $c = $area -split ',' | ForEach-Object { [int]$_ }
            for ($z = $c[1]; $z -le $c[3]; $z++) {
                $process.StandardInput.WriteLine("fill $($c[0] * 16) 319 $($z * 16) $($c[2] * 16 + 15) 319 $($z * 16 + 15) $block")
            }
        }
        Start-Sleep -Seconds 2
    }
    Start-Sleep -Seconds 3
    $process.StandardInput.WriteLine('save hold')
    Wait-For 'Saving' 60
    $saved = $false
    for ($attempt = 0; $attempt -lt 30 -and -not $saved; $attempt++) {
        $process.StandardInput.WriteLine('save query')
        try { Wait-For 'Data saved' 2; $saved = $true } catch { }
    }
    $process.StandardInput.WriteLine('save resume')
    $process.StandardInput.WriteLine('stop')
    Wait-For 'Quit correctly' 120
    $process.WaitForExit(60000) | Out-Null
}
finally {
    if (-not $process.HasExited) { $process.Kill() }
    Unregister-Event -SourceIdentifier $subscription.Name
}
Write-Host "world db: $(Join-Path $world 'db')"
