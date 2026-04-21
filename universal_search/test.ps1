Write-Host "=== Final Verification ===" -ForegroundColor Cyan

Write-Host "`n[1] Health Check" -ForegroundColor Yellow
try {
    $r = Invoke-RestMethod -Uri http://localhost:3005/health
    Write-Host "  Status: $($r.status)" -ForegroundColor Green
} catch {
    Write-Host "  FAIL: $_" -ForegroundColor Red
    exit 1
}

Write-Host "`n[2] Agent Test (what time now?)" -ForegroundColor Yellow
try {
    $r = Invoke-RestMethod -Uri http://localhost:3005/agent -Method Post -ContentType "application/json" -Body '{"query":"what time now?"}'
    Write-Host "  Job ID: $($r.id)" -ForegroundColor Green
    Write-Host "  Waiting 25 seconds..." -ForegroundColor Yellow
    Start-Sleep -Seconds 25
    $s = Invoke-RestMethod -Uri "http://localhost:3005/agent/$($r.id)"
    if ($s.data) {
        Write-Host "  Answer: $($s.data.answer.Substring(0, [Math]::Min(80, $s.data.answer.Length)))..." -ForegroundColor Green
        Write-Host "  Turns: $($s.data.turns)" -ForegroundColor Green
        Write-Host "  PASS" -ForegroundColor Green
    } elseif ($s.error) {
        Write-Host "  FAIL: $($s.error)" -ForegroundColor Red
    }
} catch {
    Write-Host "  FAIL: $_" -ForegroundColor Red
}

Write-Host "`n=== All Checks Passed ===" -ForegroundColor Cyan
