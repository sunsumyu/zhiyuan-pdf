#!/bin/bash
# Extract frames at multiple time points from the MP4 using GStreamer.
# Video: 7.767s, 2560x1360, H.264, 30fps (~233 frames).
# Target timestamps: 0%, 25%, 50%, 75% -> 0.0s, 1.94s, 3.88s, 5.83s
set -e

SRC="C:/Users/Aren/Documents/ShareX/Screenshots/2026-09/pdf-viewer-standalone_gPK7191p84.mp4"
OUTDIR="C:/temp/frames"
mkdir -p "$OUTDIR"

echo "=== GStreamer element sanity check ==="
gst-inspect-1.0 qtdemux   > /dev/null 2>&1 && echo "qtdemux: OK"    || echo "qtdemux: MISSING"
gst-inspect-1.0 d3d11h264dec > /dev/null 2>&1 && echo "d3d11h264dec: OK" || echo "d3d11h264dec: MISSING"
gst-inspect-1.0 videoconvert > /dev/null 2>&1 && echo "videoconvert: OK" || echo "videoconvert: MISSING"
gst-inspect-1.0 pngenc > /dev/null 2>&1 && echo "pngenc: OK" || echo "pngenc: MISSING"
gst-inspect-1.0 multifilesink > /dev/null 2>&1 && echo "multifilesink: OK" || echo "multifilesink: MISSING"
gst-inspect-1.0 videorate > /dev/null 2>&1 && echo "videorate: OK" || echo "videorate: MISSING"

echo ""
echo "=== Approach A: videorate 1/30 (one frame every 30s, i.e. only at 0s) ==="
gst-launch-1.0 -q \
  filesrc location="$SRC" \
  ! qtdemux name=demux \
  demux.video_0 \
  ! d3d11h264dec \
  ! d3d11download \
  ! videoconvert \
  ! videorate \
  ! video/x-raw,framerate=1/1 \
  ! pngenc \
  ! multifilesink location="$OUTDIR/vr_all_%02d.png" \
  2>&1 | tail -8
echo "exit=$?"
ls -la "$OUTDIR"
