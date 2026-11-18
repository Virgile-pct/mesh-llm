# Runs the restore block of build-windows.ps1 against a caller environment
# holding an empty variable, after a "build" that changes it and adds another.
$ErrorActionPreference = 'Stop'
[System.Environment]::SetEnvironmentVariable('MESH_T_EMPTY', '', 'Process')
$env:MESH_T_KEEP = 'keep'
$callerEnvironment = @{}
foreach ($variable in Get-ChildItem env:) { $callerEnvironment[$variable.Name] = $variable.Value }
"caller has MESH_T_EMPTY: $($callerEnvironment.ContainsKey('MESH_T_EMPTY'))"
$env:MESH_T_EMPTY = 'true'
$env:MESH_T_NEW = 'created'
$env:MESH_T_KEEP = 'changed'

$buildEnvironment = @{}
foreach ($variable in Get-ChildItem env:) { $buildEnvironment[$variable.Name] = $variable.Value }
foreach ($name in $buildEnvironment.Keys) {
    if (-not $callerEnvironment.ContainsKey($name)) {
        Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    }
}
foreach ($entry in $callerEnvironment.GetEnumerator()) {
    $unchanged = $buildEnvironment.ContainsKey($entry.Key) -and $buildEnvironment[$entry.Key] -ceq $entry.Value
    if (-not $unchanged) {
        [System.Environment]::SetEnvironmentVariable($entry.Key, [string]$entry.Value, "Process")
    }
}
$after = @{}
foreach ($variable in Get-ChildItem env:) { $after[$variable.Name] = $variable.Value }
"PowerShell $($PSVersionTable.PSVersion)"
"MESH_T_NEW present: $($after.ContainsKey('MESH_T_NEW'))"
"MESH_T_KEEP: $($after['MESH_T_KEEP'])"
"MESH_T_EMPTY present: $($after.ContainsKey('MESH_T_EMPTY')) value: '$($after['MESH_T_EMPTY'])'"
$diff = @($callerEnvironment.Keys | Where-Object { -not $after.ContainsKey($_) -or $after[$_] -cne $callerEnvironment[$_] })
$extra = @($after.Keys | Where-Object { -not $callerEnvironment.ContainsKey($_) })
"differences from the caller: $($diff.Count) changed or missing, $($extra.Count) extra"
