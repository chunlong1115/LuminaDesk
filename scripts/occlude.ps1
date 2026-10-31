# S4 occlusion test: TopMost black window covering the container area for N seconds.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File occlude.ps1 [seconds=60]
# While it runs, watch the app log for diag ticks (S4: ticks must continue 1/s).
param([int]$Seconds = 60)
Add-Type -AssemblyName System.Windows.Forms
$f = New-Object System.Windows.Forms.Form
$f.FormBorderStyle = 'None'
$f.StartPosition = 'Manual'
$f.Location = New-Object System.Drawing.Point(0, 0)
$f.Size = New-Object System.Drawing.Size(800, 620)
$f.BackColor = [System.Drawing.Color]::Black
$f.TopMost = $true
$f.ShowInTaskbar = $false
$f.Text = 'lumina-occluder'
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $Seconds * 1000
$timer.Add_Tick({ $f.Close() })
$timer.Start()
$f.Add_Shown({ $f.Activate() })
[void]$f.ShowDialog()
