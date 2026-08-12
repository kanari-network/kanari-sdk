param(
    [Parameter(Mandatory = $true)]
    [string]$KanariExe,

    [Parameter(Mandatory = $true)]
    [string]$KeystorePath,

    [Parameter(Mandatory = $true)]
    [string]$Sender,

    [Parameter(Mandatory = $true)]
    [string]$Recipient,

    [Parameter(Mandatory = $true)]
    [double]$Amount,

    [Parameter(Mandatory = $true)]
    [int]$Count,

    [Parameter(Mandatory = $true)]
    [string]$Rpc,

    [ValidateRange(20, 7200)]
    [int]$CommitTimeoutSec = 60
)

$ErrorActionPreference = 'Stop'
$env:KANARI_KEYSTORE_PATH = $KeystorePath
$password = $env:KANARI_LOAD_PASSWORD
if ([string]::IsNullOrWhiteSpace($password)) {
    throw 'KANARI_LOAD_PASSWORD is required.'
}

& $KanariExe `
    client `
    stress-test `
    --from $Sender `
    --to $Recipient `
    --amount $Amount `
    --count $Count `
    --commit-timeout-sec $CommitTimeoutSec `
    --rpc $Rpc `
    -p $password

exit $LASTEXITCODE
