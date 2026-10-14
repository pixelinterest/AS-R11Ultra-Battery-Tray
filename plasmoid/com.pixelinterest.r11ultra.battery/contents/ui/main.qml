import QtQuick
import QtQuick.Layouts
import QtCore

import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasma5support as Plasma5Support
import org.kde.plasma.plasmoid

PlasmoidItem {
    id: root

    readonly property int pollInterval: 30000
    readonly property string command: {
        var home = String(StandardPaths.writableLocation(StandardPaths.HomeLocation))
        if (home.startsWith("file://")) {
            home = decodeURIComponent(home.slice(7))
        }
        return "'" + home.replace(/'/g, "'\\''") + "/.local/bin/R11UltraBattery' --json"
    }

    readonly property bool onDesktop: Plasmoid.formFactor === PlasmaCore.Types.Planar
    readonly property int pixelScale: onDesktop ? 4 : 2
    readonly property int pixelPad: onDesktop ? 8 : 4
    readonly property string readoutText: ok ? String(percent) + "%" : "--"
    readonly property int chipWidth: (3 * readoutText.length + Math.max(0, readoutText.length - 1)) * pixelScale + pixelPad * 2
    readonly property int chipHeight: 5 * pixelScale + pixelPad * 2

    property bool ok: false
    property int percent: 0
    property bool charging: false
    property bool wired: false
    property var voltageMv: null
    property string errorText: ""

    Layout.minimumWidth: chipWidth
    Layout.minimumHeight: chipHeight
    Layout.preferredWidth: chipWidth
    Layout.preferredHeight: chipHeight
    Layout.maximumWidth: chipWidth
    Layout.maximumHeight: chipHeight

    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    Plasmoid.icon: "input-mouse"
    preferredRepresentation: compactRepresentation
    switchWidth: chipWidth
    switchHeight: chipHeight

    toolTipMainText: i18n("R11 Ultra")
    toolTipSubText: {
        if (!ok) {
            return errorText.length > 0 ? errorText : i18n("No battery reading")
        }
        var state = charging ? i18n("Charging") : (wired ? i18n("Wired") : i18n("Discharging"))
        var line = percent + "% · " + state
        if (voltageMv !== null && voltageMv !== undefined) {
            line += " · " + voltageMv + " mV"
        }
        return line
    }

    function digitColor() {
        if (!ok) {
            return "#AAAAAA"
        }
        if (charging) {
            return "#3498DB"
        }
        if (percent >= 50) {
            return "#2ECC71"
        }
        if (percent >= 20) {
            return "#E67E22"
        }
        return "#E74C3C"
    }

    function applyReading(stdout) {
        var line = (stdout || "").trim()
        if (line.length === 0) {
            return
        }
        var reading = null
        try {
            reading = JSON.parse(line)
        } catch (err) {
            ok = false
            errorText = i18n("Bad battery response")
            return
        }
        if (!reading.ok) {
            ok = false
            errorText = i18n("Mouse not found")
            return
        }
        ok = true
        percent = reading.percent
        charging = !!reading.charging
        wired = !!reading.wired
        voltageMv = reading.voltage_mv
        errorText = ""
    }

    Plasma5Support.DataSource {
        id: batterySource
        engine: "executable"
        connectedSources: [root.command]
        interval: root.pollInterval

        onNewData: function(sourceName, data) {
            root.applyReading(data["stdout"])
            var stderr = (data["stderr"] || "").trim()
            if (!root.ok && stderr.length > 0) {
                root.errorText = stderr.split("\n")[0]
            }
        }
    }

    component PixelReadout: Item {
        id: readout

        property string label: "100%"
        property color glyphColor: "#2ECC71"
        property int scale: 4
        property int pad: 8

        implicitWidth: (3 * label.length + Math.max(0, label.length - 1)) * scale + pad * 2
        implicitHeight: 5 * scale + pad * 2

        Rectangle {
            anchors.fill: parent
            radius: 8
            color: Qt.rgba(28 / 255, 28 / 255, 28 / 255, 0.3)
        }

        Canvas {
            id: pixels
            anchors.fill: parent

            onPaint: {
                var ctx = getContext("2d")
                var text = readout.label
                var scale = readout.scale
                var pad = readout.pad
                var w = width
                var h = height
                ctx.clearRect(0, 0, w, h)

                var glyphs = {
                    "0": [0b111, 0b101, 0b101, 0b101, 0b111],
                    "1": [0b010, 0b110, 0b010, 0b010, 0b111],
                    "2": [0b111, 0b001, 0b111, 0b100, 0b111],
                    "3": [0b111, 0b001, 0b111, 0b001, 0b111],
                    "4": [0b101, 0b101, 0b111, 0b001, 0b001],
                    "5": [0b111, 0b100, 0b111, 0b001, 0b111],
                    "6": [0b111, 0b100, 0b111, 0b101, 0b111],
                    "7": [0b111, 0b001, 0b001, 0b001, 0b001],
                    "8": [0b111, 0b101, 0b111, 0b101, 0b111],
                    "9": [0b111, 0b101, 0b111, 0b001, 0b111],
                    "%": [0b101, 0b001, 0b010, 0b100, 0b101],
                    "-": [0b000, 0b000, 0b111, 0b000, 0b000]
                }

                var mask = new Array(w * h)
                for (var i = 0; i < mask.length; i++) {
                    mask[i] = 0
                }
                for (var gi = 0; gi < text.length; gi++) {
                    var rows = glyphs[text.charAt(gi)]
                    if (!rows) {
                        continue
                    }
                    var gx = pad + gi * 4 * scale
                    for (var row = 0; row < 5; row++) {
                        for (var col = 0; col < 3; col++) {
                            if ((rows[row] & (0b100 >> col)) === 0) {
                                continue
                            }
                            for (var dy = 0; dy < scale; dy++) {
                                for (var dx = 0; dx < scale; dx++) {
                                    var x = gx + col * scale + dx
                                    var y = pad + row * scale + dy
                                    if (x >= 0 && y >= 0 && x < w && y < h) {
                                        mask[y * w + x] = 1
                                    }
                                }
                            }
                        }
                    }
                }

                ctx.fillStyle = "rgba(18,18,18,0.92)"
                for (var oy = 0; oy < h; oy++) {
                    for (var ox = 0; ox < w; ox++) {
                        if (mask[oy * w + ox]) {
                            continue
                        }
                        var touch = false
                        for (var ny = oy - 1; ny <= oy + 1 && !touch; ny++) {
                            for (var nx = ox - 1; nx <= ox + 1; nx++) {
                                if (nx >= 0 && ny >= 0 && nx < w && ny < h && mask[ny * w + nx]) {
                                    touch = true
                                    break
                                }
                            }
                        }
                        if (touch) {
                            ctx.fillRect(ox, oy, 1, 1)
                        }
                    }
                }

                ctx.fillStyle = readout.glyphColor
                for (var py = 0; py < h; py++) {
                    for (var px = 0; px < w; px++) {
                        if (mask[py * w + px]) {
                            ctx.fillRect(px, py, 1, 1)
                        }
                    }
                }
            }
        }

        onLabelChanged: pixels.requestPaint()
        onGlyphColorChanged: pixels.requestPaint()
        onScaleChanged: pixels.requestPaint()
        onWidthChanged: pixels.requestPaint()
        onHeightChanged: pixels.requestPaint()
        Component.onCompleted: pixels.requestPaint()
    }

    compactRepresentation: Item {
        implicitWidth: root.chipWidth
        implicitHeight: root.chipHeight

        PixelReadout {
            anchors.centerIn: parent
            width: root.chipWidth
            height: root.chipHeight
            label: root.readoutText
            glyphColor: root.digitColor()
            scale: root.pixelScale
            pad: root.pixelPad
        }
    }

    fullRepresentation: Item {
        implicitWidth: root.chipWidth
        implicitHeight: root.chipHeight

        PixelReadout {
            anchors.centerIn: parent
            width: root.chipWidth
            height: root.chipHeight
            label: root.readoutText
            glyphColor: root.digitColor()
            scale: root.pixelScale
            pad: root.pixelPad
        }
    }
}
