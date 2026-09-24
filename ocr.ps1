Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType=WindowsRuntime]

$asyncAction = [Windows.Foundation.AsyncOperation]::GetAwaiter()

function Get-Ocr($path) {
    $decoderTask = [Windows.Storage.StorageFile]::GetFileFromPathAsync($path)
    $file = $asyncAction.GetResult($decoderTask.GetCompleted())
    $imgDecoderTask = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($file)
    $decoder = $asyncAction.GetResult($imgDecoderTask.GetCompleted())
    $bitmapTask = $decoder.GetSoftwareBitmapAsync()
    $bitmap = $asyncAction.GetResult($bitmapTask.GetCompleted())
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguage()
    if ($null -eq $engine) { $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('en-US')) }
    $result = $asyncAction.GetResult($engine.RecognizeAsync($bitmap).GetCompleted())
    $result.Text
}

$files = Get-ChildItem -Path "C:\temp\frames\frame_*.png"
Write-Host "Processing $($files.Count) frames"
foreach ($f in $files) {
    Write-Output "========== $($f.Name) =========="
    try { Get-Ocr $f.FullName } catch { Write-Output "OCR ERROR: $_" }
    Write-Output ""
}
