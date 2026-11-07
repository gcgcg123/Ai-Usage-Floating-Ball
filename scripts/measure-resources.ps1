param(
    [string]$ProcessName = "ai-usage-liquidball-tauri",
    [int]$DurationSeconds = 60,
    [int]$IntervalMilliseconds = 1000,
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $resultDirectory = Join-Path $PSScriptRoot "..\results"
    New-Item -ItemType Directory -Force -Path $resultDirectory | Out-Null
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputPath = Join-Path $resultDirectory "resource-$timestamp.csv"
}

function Get-ProcessTreeIds {
    param([int[]]$RootIds)

    $allProcesses = Get-CimInstance Win32_Process |
        Select-Object ProcessId, ParentProcessId
    $wanted = [System.Collections.Generic.HashSet[int]]::new()

    foreach ($id in $RootIds) {
        [void]$wanted.Add($id)
    }

    $added = $true
    while ($added) {
        $added = $false
        foreach ($process in $allProcesses) {
            if (
                $wanted.Contains([int]$process.ParentProcessId) -and
                -not $wanted.Contains([int]$process.ProcessId)
            ) {
                [void]$wanted.Add([int]$process.ProcessId)
                $added = $true
            }
        }
    }

    return @($wanted)
}

$roots = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)
if ($roots.Count -eq 0) {
    throw "Process '$ProcessName' is not running."
}

$logicalCpuCount = [Environment]::ProcessorCount
$samples = [System.Collections.Generic.List[object]]::new()
$endAt = (Get-Date).AddSeconds($DurationSeconds)
$previousCpuSeconds = $null
$previousSampleAt = $null

while ((Get-Date) -lt $endAt) {
    $sampleAt = Get-Date
    $currentRootIds = @(
        Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty Id
    )

    if ($currentRootIds.Count -eq 0) {
        break
    }

    $treeIds = Get-ProcessTreeIds -RootIds $currentRootIds
    $processes = @(
        foreach ($id in $treeIds) {
            Get-Process -Id $id -ErrorAction SilentlyContinue
        }
    )

    $workingSet = ($processes | Measure-Object WorkingSet64 -Sum).Sum
    $privateMemory = ($processes | Measure-Object PrivateMemorySize64 -Sum).Sum
    $cpuSeconds = ($processes | Measure-Object CPU -Sum).Sum

    $cpuPercent = $null
    if ($null -ne $previousCpuSeconds -and $null -ne $previousSampleAt) {
        $elapsedSeconds = ($sampleAt - $previousSampleAt).TotalSeconds
        if ($elapsedSeconds -gt 0) {
            $cpuPercent = (($cpuSeconds - $previousCpuSeconds) /
                ($elapsedSeconds * $logicalCpuCount)) * 100
        }
    }

    $samples.Add([pscustomobject]@{
        Timestamp = $sampleAt.ToString("o")
        RootProcessCount = $currentRootIds.Count
        ProcessTreeCount = $processes.Count
        WorkingSetMB = [Math]::Round($workingSet / 1MB, 2)
        PrivateMemoryMB = [Math]::Round($privateMemory / 1MB, 2)
        CpuPercent = if ($null -eq $cpuPercent) {
            $null
        } else {
            [Math]::Round($cpuPercent, 3)
        }
    })

    $previousCpuSeconds = $cpuSeconds
    $previousSampleAt = $sampleAt
    Start-Sleep -Milliseconds $IntervalMilliseconds
}

$samples | Export-Csv -LiteralPath $OutputPath -NoTypeInformation -Encoding UTF8

$withCpu = @($samples | Where-Object { $null -ne $_.CpuPercent })
$summary = [pscustomobject]@{
    OutputPath = (Resolve-Path -LiteralPath $OutputPath).Path
    Samples = $samples.Count
    AverageWorkingSetMB = [Math]::Round(
        ($samples | Measure-Object WorkingSetMB -Average).Average, 2)
    PeakWorkingSetMB = [Math]::Round(
        ($samples | Measure-Object WorkingSetMB -Maximum).Maximum, 2)
    AveragePrivateMemoryMB = [Math]::Round(
        ($samples | Measure-Object PrivateMemoryMB -Average).Average, 2)
    PeakPrivateMemoryMB = [Math]::Round(
        ($samples | Measure-Object PrivateMemoryMB -Maximum).Maximum, 2)
    AverageCpuPercent = if ($withCpu.Count -gt 0) {
        [Math]::Round(($withCpu | Measure-Object CpuPercent -Average).Average, 3)
    } else {
        $null
    }
    PeakCpuPercent = if ($withCpu.Count -gt 0) {
        [Math]::Round(($withCpu | Measure-Object CpuPercent -Maximum).Maximum, 3)
    } else {
        $null
    }
    MaxProcessTreeCount = (
        $samples | Measure-Object ProcessTreeCount -Maximum
    ).Maximum
}

$summary | Format-List
