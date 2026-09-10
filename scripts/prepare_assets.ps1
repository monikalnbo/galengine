Add-Type -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Collections.Generic;

public class ImageHelper {
    public static void ResizeBg(string srcPath, string destPath) {
        using (var src = Image.FromFile(srcPath))
        using (var bmp = new Bitmap(1280, 720)) {
            using (var g = Graphics.FromImage(bmp)) {
                g.InterpolationMode = System.Drawing.Drawing2D.InterpolationMode.HighQualityBicubic;
                g.DrawImage(src, 0, 0, 1280, 720);
            }
            bmp.Save(destPath, ImageFormat.Jpeg);
        }
    }

    public static void ProcessSprite(string srcPath, string destPath) {
        using (var src = new Bitmap(srcPath)) {
            int w = src.Width;
            int h = src.Height;
            bool[,] isBg = new bool[w, h];
            var queue = new Queue<Point>();

            for (int x = 0; x < w; x++) {
                queue.Enqueue(new Point(x, 0));
                queue.Enqueue(new Point(x, h - 1));
                isBg[x, 0] = true;
                isBg[x, h - 1] = true;
            }
            for (int y = 0; y < h; y++) {
                queue.Enqueue(new Point(0, y));
                queue.Enqueue(new Point(w - 1, y));
                isBg[0, y] = true;
                isBg[w - 1, y] = true;
            }

            int[] dx = { 0, 0, 1, -1 };
            int[] dy = { 1, -1, 0, 0 };

            while (queue.Count > 0) {
                Point p = queue.Dequeue();
                Color c = src.GetPixel(p.X, p.Y);
                if (c.R >= 225 && c.G >= 225 && c.B >= 225) {
                    for (int i = 0; i < 4; i++) {
                        int nx = p.X + dx[i];
                        int ny = p.Y + dy[i];
                        if (nx >= 0 && nx < w && ny >= 0 && ny < h && !isBg[nx, ny]) {
                            isBg[nx, ny] = true;
                            Color nc = src.GetPixel(nx, ny);
                            if (nc.R >= 215 && nc.G >= 215 && nc.B >= 215) {
                                queue.Enqueue(new Point(nx, ny));
                            }
                        }
                    }
                }
            }

            using (var cutout = new Bitmap(w, h, PixelFormat.Format32bppArgb)) {
                for (int y = 0; y < h; y++) {
                    for (int x = 0; x < w; x++) {
                        Color c = src.GetPixel(x, y);
                        if (isBg[x, y] && c.R >= 215 && c.G >= 215 && c.B >= 215) {
                            cutout.SetPixel(x, y, Color.FromArgb(0, 0, 0, 0));
                        } else {
                            cutout.SetPixel(x, y, c);
                        }
                    }
                }

                using (var canvas = new Bitmap(1280, 720, PixelFormat.Format32bppArgb)) {
                    using (var g = Graphics.FromImage(canvas)) {
                        g.InterpolationMode = System.Drawing.Drawing2D.InterpolationMode.HighQualityBicubic;
                        int targetH = 680;
                        int targetW = (int)((double)w * targetH / h);
                        int posX = (1280 - targetW) / 2;
                        int posY = 720 - targetH;
                        g.DrawImage(cutout, posX, posY, targetW, targetH);
                    }
                    canvas.Save(destPath, ImageFormat.Png);
                }
            }
        }
    }
}
'@ -ReferencedAssemblies System.Drawing

$brain = "C:\Users\27258\.gemini\antigravity-ide\brain\be6c1e9e-b25a-4c98-b1fd-89c5c922eaf4"
$templateData = "d:\for_clone\galengine\template\game\data"
$rootData = "d:\for_clone\galengine\game\data"

$bgDirs = @(
    "$templateData\bgimage",
    "$templateData\bg",
    "$rootData\bgimage",
    "$rootData\bg"
)

$charDirs = @(
    "$templateData\fgimage",
    "$templateData\char",
    "$rootData\fgimage",
    "$rootData\char"
)

$cgDirs = @(
    "$templateData\cg",
    "$rootData\cg"
)

foreach ($d in ($bgDirs + $charDirs + $cgDirs)) {
    if (-not (Test-Path $d)) {
        New-Item -ItemType Directory -Force -Path $d | Out-Null
    }
}

function Save-AllBg($srcName, $destName) {
    $src = Join-Path $brain $srcName
    Write-Host "Resizing BG: $destName"
    foreach ($d in $bgDirs) {
        $p = Join-Path $d $destName
        [ImageHelper]::ResizeBg($src, $p)
    }
    if ($destName.StartsWith("cg_")) {
        foreach ($d in $cgDirs) {
            $p = Join-Path $d $destName
            [ImageHelper]::ResizeBg($src, $p)
        }
    }
}

Save-AllBg "title_screen_bg_1789059883964.jpg" "title.jpg"
Save-AllBg "title_screen_bg_1789059883964.jpg" "bg_space.jpg"
Save-AllBg "bg_school_cherry_1789059926044.jpg" "school.jpg"
Save-AllBg "bg_sunset_class_1789059962921.jpg" "classroom.jpg"
Save-AllBg "cg_alice_sunset_1789059981956.jpg" "cg_alice.jpg"

function Save-AllSprite($srcName, $destName) {
    $src = Join-Path $brain $srcName
    Write-Host "Processing Sprite: $destName"
    $tempOut = "$env:TEMP\$destName"
    [ImageHelper]::ProcessSprite($src, $tempOut)
    foreach ($d in $charDirs) {
        $p = Join-Path $d $destName
        Copy-Item $tempOut $p -Force
        Write-Host "Copied sprite to $p"
    }
    Remove-Item $tempOut -Force
}

$normalFile = (Get-ChildItem $brain -Filter "alice_sprite_normal_*.jpg" | Select-Object -First 1).Name
$shockFile = (Get-ChildItem $brain -Filter "alice_sprite_shock_*.jpg" | Select-Object -First 1).Name

Save-AllSprite $normalFile "alice_normal.png"
Save-AllSprite $shockFile "alice_shock.png"

Write-Host "=== All assets prepared successfully! ==="
