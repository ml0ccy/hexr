Write-Host "Testing new file creation..."
& "./target/release/hexr.exe" "test_new.bin" "--new" "256" "--pattern" "aa"
Write-Host "Program exited with code: $LASTEXITCODE"

if (Test-Path "test_new.bin") {
    Write-Host "File created successfully!"
    $size = (Get-Item "test_new.bin").Length
    Write-Host "File size: $size bytes"

    # Read first few bytes to verify pattern
    $content = Get-Content "test_new.bin" -Encoding Byte -TotalCount 10
    Write-Host "First 10 bytes: $($content -join ' ')"
} else {
    Write-Host "File was not created"
}
