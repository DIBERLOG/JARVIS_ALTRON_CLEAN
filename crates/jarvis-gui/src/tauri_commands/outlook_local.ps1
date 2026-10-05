$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    $app=$null
    try { $app=[Runtime.InteropServices.Marshal]::GetActiveObject('Outlook.Application') } catch {}
    if (!$app -and $request.action -in 'connect','show_inbox','launch','compose') {
        $paths=@()
        foreach ($key in @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\OUTLOOK.EXE','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths\OUTLOOK.EXE')) {
            if (Test-Path $key) { $paths += (Get-Item $key).GetValue('') }
        }
        foreach ($root in @($env:ProgramFiles,${env:ProgramFiles(x86)})) {
            if ($root) { $paths += "$root\Microsoft Office\root\Office16\OUTLOOK.EXE"; $paths += "$root\Microsoft Office\Office16\OUTLOOK.EXE" }
        }
        $exe=$paths | Where-Object { $_ -and [IO.Path]::GetFileName($_) -ieq 'OUTLOOK.EXE' -and (Test-Path -LiteralPath $_ -PathType Leaf) } | Select-Object -First 1
        if (!$exe) { throw 'classic_missing' }
        if ($request.profile) {
            if ($request.profile -match '["\r\n]' -or $request.profile.Length -gt 256) { throw 'profile_invalid' }
            Start-Process -FilePath $exe -ArgumentList ('/profile "'+$request.profile+'"')
        } else { Start-Process -FilePath $exe }
        $deadline=[DateTime]::UtcNow.AddSeconds(30)
        while (!$app -and [DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 400
            try { $app=[Runtime.InteropServices.Marshal]::GetActiveObject('Outlook.Application') } catch {}
        }
        if (!$app) { throw 'classic_not_ready' }
    }
    if (!$app) { throw 'classic_closed' }
    if ($request.action -eq 'launch') {
        $explorer=$app.ActiveExplorer()
        if ($explorer) { $explorer.Display(); $explorer.Activate() }
        @{shown=$true} | ConvertTo-Json -Compress
        exit 0
    }
    $session = $app.Session
    function AccountEmail($entry) {
        if ($entry.SmtpAddress) { return [string]$entry.SmtpAddress }
        # Kerio exposes an olOtherAccount without SmtpAddress; its connector labels
        # the account as name(full@address). Never guess from a store/user name.
        if ($entry.AccountType -eq 5 -and $entry.DisplayName -match '\(([^()\s]+@[^()\s]+)\)$') { return $Matches[1] }
        return ''
    }
    $account = @($session.Accounts | Where-Object { (AccountEmail $_) -eq $request.email })
    if ($request.action -eq 'connect') {
        $accounts = @($session.Accounts | ForEach-Object { @{name=$_.DisplayName;email=(AccountEmail $_)} })
        if ($accounts.Count -ne 1 -or !$accounts[0].email) { throw 'Select a profile with exactly one mail account.' }
        $result = @{connected=$true;account=$accounts[0];profile=$session.CurrentProfileName}
    } else {
        if ($account.Count -ne 1 -or $session.CurrentProfileName -ne $request.profile) { throw 'Outlook account or profile changed. Reconnect.' }
        $store = $account[0].DeliveryStore
        function MailData($mail, $full) {
            $address = $mail.SenderEmailAddress
            if ($mail.SenderEmailType -eq 'EX') {
                try { $address = $mail.Sender.GetExchangeUser().PrimarySmtpAddress } catch { $address = '' }
            }
            $body = [string]$mail.Body
            $data = @{id=($store.StoreID + ':' + $mail.EntryID);subject=$mail.Subject;from=@{emailAddress=@{name=$mail.SenderName;address=$address}};receivedDateTime=$mail.ReceivedTime.ToUniversalTime().ToString('o');isRead=(!$mail.UnRead);bodyPreview=$body.Substring(0,[Math]::Min(200,$body.Length))}
            if ($full) { $data.body=@{contentType='Text';content=$body.Substring(0,[Math]::Min(100000,$body.Length))} }
            return $data
        }
        switch ($request.action) {
            'recipient_suggestions' {
                $items=$store.GetDefaultFolder(5).Items
                $items.Sort('[SentOn]',$true)
                $query=([string]$request.message.name).ToLowerInvariant().Replace([char]0x0451,[char]0x0435)
                $seen=@{}; $candidates=@(); $scanned=0
                for ($i=1; $i -le [Math]::Min(100,$items.Count) -and $candidates.Count -lt 5 -and $scanned -lt 500; $i++) {
                    $sent=$items.Item($i)
                    if ($sent.Class -ne 43) { continue }
                    foreach ($entry in $sent.Recipients) {
                        $scanned++
                        if ($scanned -gt 500) { break }
                        if ($entry.Type -ne 1) { continue }
                        try {
                            if ($entry.AddressEntry.AddressEntryUserType -in 1,11) { continue }
                            $address=[string]$entry.Address
                            if ($entry.AddressEntry.Type -eq 'EX') { $address=[string]$entry.AddressEntry.GetExchangeUser().PrimarySmtpAddress }
                            if ($address -notmatch '^[^\s<>;,]+@[^\s<>;,]+\.[^\s<>;,]+$') { continue }
                            $key=$address.ToLowerInvariant()
                            $label=([string]$entry.Name).ToLowerInvariant().Replace([char]0x0451,[char]0x0435)
                            if ($seen.ContainsKey($key) -or ($query -and !$key.Contains($query) -and !$label.Contains($query))) { continue }
                            $seen[$key]=$true
                            $candidates+=@{email=$address;name=[string]$entry.Name}
                            if ($candidates.Count -ge 5) { break }
                        } catch { continue }
                    }
                }
                $result=@{candidates=@($candidates)}
            }
            'resolve_recipient' {
                $entry=$session.CreateRecipient([string]$request.message.name)
                $null=$entry.Resolve()
                if (!$entry.Resolved) { throw 'recipient_not_found' }
                if ($entry.AddressEntry.AddressEntryUserType -in 1,11) { throw 'recipient_not_found' }
                $address=[string]$entry.Address
                if ($entry.AddressEntry.Type -eq 'EX') { $address=[string]$entry.AddressEntry.GetExchangeUser().PrimarySmtpAddress }
                if ($address -notmatch '^[^\s<>;,]+@[^\s<>;,]+\.[^\s<>;,]+$') { throw 'recipient_not_found' }
                $result=@{candidates=@(@{email=$address;name=[string]$entry.Name})}
            }
            'compose_read' {
                $mail=$session.GetItemFromID($request.message.composeId,$store.StoreID)
                if ($mail.Class -ne 43 -or $mail.Sent -or $mail.Parent.EntryID -ne $store.GetDefaultFolder(16).EntryID -or $mail.Attachments.Count -gt 0 -or $mail.BodyFormat -ne 1 -or @($mail.Recipients | Where-Object {$_.Type -ne 1}).Count -gt 0) { throw 'draft_changed' }
                $mail.Save()
                if ($mail.Recipients.Count -eq 0 -or !$mail.Recipients.ResolveAll()) { throw 'recipient_missing' }
                $mail.Save()
                $addresses=@($mail.Recipients | ForEach-Object {
                    $address=[string]$_.Address
                    if ($_.AddressEntry.Type -eq 'EX') { $address=[string]$_.AddressEntry.GetExchangeUser().PrimarySmtpAddress }
                    $address
                })
                $result=@{draft=@{to=($addresses -join ';');subject=[string]$mail.Subject;body=[string]$mail.Body}}
            }
            'compose' {
                $mail=$store.GetDefaultFolder(16).Items.Add('IPM.Note')
                $mail.SendUsingAccount=$account[0]
                $mail.BodyFormat=1
                $mail.Save()
                $mail.Display(); $mail.GetInspector.Activate()
                $mail.Body=''; $mail.Save()
                $result=@{shown=$true;nativeId=$mail.EntryID}
            }
            'show_inbox' {
                $folder=$store.GetDefaultFolder(6)
                $explorer=$app.ActiveExplorer()
                if (!$explorer) { $explorer=$folder.GetExplorer() }
                $explorer.CurrentFolder=$folder
                $explorer.Display()
                $explorer.Activate()
                $result=@{shown=$true}
            }
            'inbox' {
                $items=$store.GetDefaultFolder(6).Items
                $items.Sort('[ReceivedTime]', $true)
                $messages=@()
                for ($i=1; $i -le [Math]::Min(50,$items.Count); $i++) {
                    $mail=$items.Item($i)
                    if ($mail.Class -eq 43) { $messages += (MailData $mail $false) }
                }
                $result=@{messages=$messages;hasMore=($items.Count -gt 50)}
            }
            { $_ -in 'message','show_message' } {
                $ids=$request.id -split ':',2
                if ($ids.Count -ne 2 -or $ids[0] -ne $store.StoreID) { throw 'Message does not belong to selected store.' }
                $mail=$session.GetItemFromID($ids[1],$store.StoreID)
                if ($mail.Class -ne 43) { throw 'Not a mail message.' }
                $result=MailData $mail $true
                if ($request.action -eq 'show_message') { $mail.Display(); $mail.GetInspector.Activate() }
            }
            { $_ -in 'draft','send','preview' } {
                if ($request.action -eq 'send' -and $request.message.nativeId) {
                    $mail=$session.GetItemFromID($request.message.nativeId,$store.StoreID)
                    $expected=@($request.message.toRecipients | ForEach-Object {$_.emailAddress.address.ToLowerInvariant()})
                    $actual=@($mail.Recipients | ForEach-Object {$_.Address.ToLowerInvariant()})
                    if ($mail.Sent -or $mail.Parent.EntryID -ne $store.GetDefaultFolder(16).EntryID -or $mail.Subject -cne $request.message.subject -or ($mail.Body -replace '\r\n','\n') -cne ($request.message.body.content -replace '\r\n','\n') -or ($actual -join ';') -cne ($expected -join ';') -or @($mail.Recipients | Where-Object {$_.Type -ne 1}).Count -gt 0 -or $mail.Attachments.Count -gt 0 -or $mail.BodyFormat -ne 1) { throw 'draft_changed' }
                    $mail.SendUsingAccount=$account[0]
                    $mail.Send()
                    $result=@{accepted=$true}
                    break
                }
                if ($request.action -eq 'preview' -and $request.message.composeId) {
                    $mail=$session.GetItemFromID($request.message.composeId,$store.StoreID)
                    if ($mail.Class -ne 43 -or $mail.Sent -or $mail.Parent.EntryID -ne $store.GetDefaultFolder(16).EntryID -or $mail.Attachments.Count -gt 0 -or $mail.BodyFormat -ne 1) { throw 'draft_changed' }
                    # Compare the last JARVIS snapshot before applying dictated corrections.
                    if ($mail.Subject -or $mail.Body -or $mail.Recipients.Count -gt 0) {
                        if (!$request.message.previous) { throw 'draft_changed' }
                        $previous=$request.message.previous
                        $expected=@($previous.toRecipients | ForEach-Object {$_.emailAddress.address.ToLowerInvariant()})
                        $actual=@($mail.Recipients | ForEach-Object {$_.Address.ToLowerInvariant()})
                        if ($mail.Subject -cne $previous.subject -or ($mail.Body -replace '\r\n','\n') -cne ($previous.body.content -replace '\r\n','\n') -or ($actual -join ';') -cne ($expected -join ';') -or @($mail.Recipients | Where-Object {$_.Type -ne 1}).Count -gt 0) { throw 'draft_changed' }
                    }
                    elseif ($request.message.previous) { throw 'draft_changed' }
                    while ($mail.Recipients.Count -gt 0) { $mail.Recipients.Remove(1) }
                } else { $mail=$store.GetDefaultFolder(16).Items.Add('IPM.Note') }
                $mail.SendUsingAccount=$account[0]
                $mail.BodyFormat=1
                $mail.Subject=$request.message.subject
                $mail.Body=$request.message.body.content
                foreach ($entry in $request.message.toRecipients) {
                    $recipient=$mail.Recipients.Add($entry.emailAddress.address)
                    $recipient.Type=1
                }
                if (!$mail.Recipients.ResolveAll()) { throw 'Recipient could not be resolved.' }
                if ($request.action -eq 'send') { $mail.Send(); $result=@{accepted=$true} }
                else {
                    $mail.Save(); $result=@{saved=$true}
                    if ($request.action -eq 'preview') { $mail.Display(); $mail.GetInspector.Activate(); $result.nativeId=$mail.EntryID }
                }
            }
            default { throw 'Unsupported local Outlook action.' }
        }
    }
    $result | ConvertTo-Json -Depth 12 -Compress
} catch {
    if ($_.Exception.Message -in 'classic_missing','classic_not_ready','classic_closed','profile_invalid','recipient_missing','recipient_not_found') { @{error=$_.Exception.Message} | ConvertTo-Json -Compress; exit 1 }
    if ($_.Exception.Message -eq 'draft_changed') { @{error='draft_changed'} | ConvertTo-Json -Compress; exit 1 }
    # Do not print COM diagnostics containing private mail content.
    @{error='Local Outlook request failed. Keep classic Outlook open on the selected profile; check any Outlook security prompt.'} | ConvertTo-Json -Compress
    exit 1
}
