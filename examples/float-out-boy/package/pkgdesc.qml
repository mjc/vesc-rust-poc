import QtQuick 2.15

Item {
    property string pkgName: "Float Out Boy"
    property string pkgDescriptionMd: "README.md"
    property string pkgLisp: "code.lisp"
    property string pkgQml: "ui.qml"
    property bool pkgQmlIsFullscreen: false
    property string pkgOutput: "Float-Out-Boy-0.1.0.vescpkg"

    function isCompatible (fwRxParams) {
        if (fwRxParams.hwTypeStr().toLowerCase() != "vesc") {
            return false;
        }

        return true;
    }
}
