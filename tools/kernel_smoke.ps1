param(
    [string] $Address = 'ws://127.0.0.1:9712',
    [switch] $CheckCommands
)

# This check never changes settings, notes, passwords or Windows windows.
# CheckCommands plays one existing acknowledgement and opens the Center panel.
$ErrorActionPreference = 'Stop'
$socket = [System.Net.WebSockets.ClientWebSocket]::new()
$connectionLimit = [System.Threading.CancellationTokenSource]::new(8000)

function Send-Action([hashtable] $Action) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($Action | ConvertTo-Json -Compress))
    $segment = [System.ArraySegment[byte]]::new($bytes)
    $null = $socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true,
        [System.Threading.CancellationToken]::None).GetAwaiter().GetResult()
}

function Wait-Event([string] $Event, [string] $Id = '', [int] $TimeoutSeconds = 15) {
    $limit = [System.Threading.CancellationTokenSource]::new($TimeoutSeconds * 1000)
    try {
        while ($true) {
            $buffer = [byte[]]::new(65536)
            $message = [System.IO.MemoryStream]::new()
            try {
                do {
                    $part = $socket.ReceiveAsync([System.ArraySegment[byte]]::new($buffer), $limit.Token).GetAwaiter().GetResult()
                    if ($part.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { throw 'The voice process disconnected.' }
                    $message.Write($buffer, 0, $part.Count)
                } until ($part.EndOfMessage)
                $data = [System.Text.Encoding]::UTF8.GetString($message.ToArray()) | ConvertFrom-Json
            } finally { $message.Dispose() }
            if ($data.event -eq $Event -and (!$Id -or $data.id -eq $Id)) { return $data }
        }
    } finally { $limit.Dispose() }
}

try {
    $null = $socket.ConnectAsync([Uri] $Address, $connectionLimit.Token).GetAwaiter().GetResult()
    Send-Action @{action = 'ping'}
    $null = Wait-Event 'pong'
    Write-Output 'PASS: voice process responds to GUI protocol'

    Send-Action @{action = 'get_muted'}
    $state = Wait-Event 'microphone_muted'
    Write-Output ('PASS: microphone state is available (muted={0})' -f $state.muted)

    if ($CheckCommands) {
        Send-Action @{action = 'text_command'; text = 'спасибо'}
        $result = Wait-Event 'command_executed' 'jarvis_thanks'
        if (!$result.success) { throw 'Acknowledgement command failed.' }
        Write-Output 'PASS: registered command executes through ALTRON kernel'

        Send-Action @{action = 'text_command'; text = 'открой центр'}
        $center = Wait-Event 'center_command'
        if ($center.text -ne 'открой центр') { throw 'Unexpected Center request.' }
        Write-Output 'PASS: Center bridge preserves command text'
    }
} finally {
    $socket.Dispose()
    $connectionLimit.Dispose()
}
