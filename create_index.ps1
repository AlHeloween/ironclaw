$req = [System.Net.HttpWebRequest]::Create("http://localhost:3005/local/index")
$req.Method = "POST"
$req.ContentType = "application/json"
$body = '{"name":"ironclaw","path":"../../","languages":["all"],"symbols_enabled":true}'
$req.ContentLength = $body.Length
$req.Timeout = 900000
$writer = $req.GetRequestStream()
$writer.Write([System.Text.Encoding]::UTF8.GetBytes($body), 0, $body.Length)
$writer.Close()
try {
    $resp = $req.GetResponse()
    $reader = New-Object System.IO.StreamReader($resp.GetResponseStream())
    $result = $reader.ReadToEnd()
    Write-Host "HTTP $($resp.StatusCode): $result"
    $reader.Close()
} catch {
    Write-Host "Error: $_"
}
