# Use Windows.Media.Ocr via Windows Runtime
Add-Type -AssemblyName 'System.Runtime.InteropServices'
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters().Count -eq 1 -and $_.GetParameters(0).ParameterType.Name -eq 'IAsyncOperation`1' })[0]
$asTask = $asTaskGeneric.MakeGenericMethod([Windows.Storage.StorageFile])
function Await($winRtTask) {
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetGenericArguments().Count -eq 1 })[0]
    $asTaskMethod = $asTaskGeneric.MakeGenericMethod($winRtTask.GetType().GetGenericArguments())
    $task = $asTaskMethod.Invoke($null, @( $winRtTask ))
    ,$task.GetAwaiter().GetResult()
}

function Get-Ocr($path) {
    $file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($path))
    $decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($file))
    $bitmap = Await ($decoder.GetSoftwareBitmapAsync())
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguage()
    if ($null -eq $engine) { $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('en-US')) }
    $result = Await ($engine.RecognizeAsync($bitmap))
    $result.Text
}

$files = Get-ChildItem -Path "C:\temp\frames\frame_*.png"
foreach ($f in $files) {
    Write-Output "========== $($f.Name) =========="
    try { Get-Ocr $f.FullName } catch { Write-Output "OCR ERROR: $($_.Exception.Message)" }
}
