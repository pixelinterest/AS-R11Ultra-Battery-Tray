import QtQuick
import QtQuick.Layouts
import QtCore

import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents
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

    property bool ok: false
    property int percent: 0
    property bool charging: false
    property bool wired: false
    property var voltageMv: null
    property string errorText: ""

    preferredRepresentation: compactRepresentation
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

    function percentColor() {
        var background = Kirigami.Theme.backgroundColor
        var light = (0.299 * background.r + 0.587 * background.g + 0.114 * background.b) > 0.6
        if (!ok) {
            return light ? "#464646" : "#AAAAAA"
        }
        if (charging) {
            return light ? "#1565C0" : "#3498DB"
        }
        if (percent >= 50) {
            return light ? "#1E8C3C" : "#2ECC71"
        }
        if (percent >= 20) {
            return light ? "#D35400" : "#E67E22"
        }
        return light ? "#C0392B" : "#E74C3C"
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

    compactRepresentation: Item {
        implicitWidth: Math.ceil(percentLabel.implicitWidth + Kirigami.Units.smallSpacing * 2)
        implicitHeight: Math.max(percentLabel.implicitHeight, Kirigami.Units.iconSizes.small)

        PlasmaComponents.Label {
            id: percentLabel
            anchors.centerIn: parent
            text: root.ok ? root.percent + "%" : "—"
            color: root.percentColor()
            font.weight: Font.Medium
        }
    }

    fullRepresentation: ColumnLayout {
        Layout.minimumWidth: Kirigami.Units.gridUnit * 10
        Layout.minimumHeight: Kirigami.Units.gridUnit * 4

        PlasmaComponents.Label {
            Layout.alignment: Qt.AlignHCenter
            text: root.ok ? root.percent + "%" : "—"
            color: root.percentColor()
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 2
            font.weight: Font.Medium
        }

        PlasmaComponents.Label {
            Layout.alignment: Qt.AlignHCenter
            text: root.toolTipSubText
            wrapMode: Text.WordWrap
        }
    }

    Component.onCompleted: {
        Plasmoid.icon = "input-mouse"
    }
}
