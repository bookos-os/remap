"""Doble del protocolo D-Bus; se ejecuta exclusivamente en un bus privado."""
import json
import os
from pathlib import Path
from gi.repository import Gio, GLib

base = Path(os.environ['REMAP_PRUEBA_BASE'])
base.mkdir(parents=True, exist_ok=True)
sistema = os.environ.get('REMAP_MOTOR') == 'sistema'
nombre = 'org.bookos.Teclas' if sistema else 'org.bookos.Desktop'
ruta = '/org/bookos/Teclas' if sistema else '/org/bookos/Desktop'
xml = f'''<node><interface name="{nombre}">
<method name="GetCapabilities"><arg type="a{{sv}}" direction="out"/></method>
<method name="GetConfig"><arg type="s" direction="out"/></method>
<method name="GetKeyRemaps"><arg type="s" direction="out"/></method>
<method name="ApplyKeyRemaps"><arg type="s" direction="in"/><arg type="b" direction="out"/><arg type="s" direction="out"/></method>
<method name="CaptureKey"><arg type="b" direction="in"/><arg type="b" direction="out"/></method>
<signal name="KeyCaptured"><arg type="u"/><arg type="b"/></signal>
</interface></node>'''
conexion = Gio.bus_get_sync(Gio.BusType.SESSION, None)
remapeos = []
armada = False

def llamada(c, s, r, i, metodo, parametros, invocacion):
    global remapeos, armada
    valores = parametros.unpack()
    with (base / 'llamadas.jsonl').open('a') as f:
        f.write(json.dumps({'metodo': metodo, 'valores': valores}) + '\n')
    if metodo == 'GetCapabilities':
        invocacion.return_value(GLib.Variant('(a{sv})', ({'key_remap': GLib.Variant('b', True)},)))
    elif metodo == 'GetConfig':
        invocacion.return_value(GLib.Variant('(s)', (json.dumps({'teclado': 'es', 'tema_efectivo': 'oscuro'}),)))
    elif metodo == 'GetKeyRemaps':
        if (base / 'no_leer').exists():
            invocacion.return_dbus_error('org.bookos.Error.Prueba', 'lectura_no_disponible')
            return
        invocacion.return_value(GLib.Variant('(s)', (json.dumps(remapeos),)))
    elif metodo == 'ApplyKeyRemaps':
        if (base / 'rechazar').exists():
            invocacion.return_value(GLib.Variant('(bs)', (False, 'rechazado_por_prueba')))
        else:
            if not (base / 'no_confirmar').exists():
                remapeos = json.loads(valores[0])
            invocacion.return_value(GLib.Variant('(bs)', (True, '')))
    elif metodo == 'CaptureKey':
        armada = valores[0]
        (base / 'armada').write_text(str(armada))
        invocacion.return_value(GLib.Variant('(b)', (True,)))

conexion.register_object(ruta, Gio.DBusNodeInfo.new_for_xml(xml).interfaces[0], llamada, None, None)
Gio.bus_own_name_on_connection(conexion, nombre, Gio.BusNameOwnerFlags.NONE, None, None)

def emitir():
    global armada
    p = base / 'capturar.json'
    if p.exists() and armada:
        dato = json.loads(p.read_text())
        p.unlink()
        armada = False
        (base / 'armada').write_text('False')
        conexion.emit_signal(None, ruta, nombre, 'KeyCaptured', GLib.Variant('(ub)', (dato['codigo'], dato.get('copilot', False))))
    return True

GLib.timeout_add(25, emitir)
(base / 'listo').write_text('correcto')
GLib.MainLoop().run()
