"""Pruebas de la ventana propia mediante eventos GPUI; sin entrada global de sesión."""
from pathlib import Path
import json
import os
import re
import subprocess
import time
from PIL import Image

base = Path(os.environ.get('REMAP_PRUEBA_BASE', '/tmp/remap-prueba-es'))
orden = base / 'orden.json'
recibo = orden.with_suffix('.respuesta')

def enviar(**dato):
    recibo.unlink(missing_ok=True)
    temporal = orden.with_suffix('.tmp')
    temporal.write_text(json.dumps(dato))
    temporal.replace(orden)
    limite = time.monotonic() + 8
    while not recibo.exists():
        if time.monotonic() > limite:
            raise RuntimeError(f'La ventana no respondió a {dato}')
        time.sleep(.03)
    return json.loads(recibo.read_text())

def estado():
    return enviar(accion='estado')

def esperar(predicado):
    limite = time.monotonic() + 8
    while True:
        e = estado()
        if predicado(e):
            return e
        if time.monotonic() > limite:
            raise AssertionError(e)
        time.sleep(.05)

def clic(x, y):
    enviar(accion='clic', x=x, y=y)
    time.sleep(.12)

def tecla(tecla, **args):
    return enviar(accion='tecla', tecla=tecla, **args)

def captura(nombre):
    enviar(accion='activar')
    enviar(accion='mover',x=400,y=600)
    time.sleep(.65)
    ancho, alto = estado()['tamano']
    r = subprocess.run(['gdbus', 'call', '--session', '--dest', 'org.bookos.portal.desktop', '--object-path', '/org/freedesktop/portal/desktop', '--method', 'org.freedesktop.impl.portal.Screenshot.Screenshot', '/org/freedesktop/portal/desktop/request/remap/gpui', 'bookos-teclas', '', '{}'], capture_output=True, text=True, check=True, timeout=8)
    imagen = Image.open(re.search("file://([^']+)", r.stdout)[1])
    # Escala medida: 2. El compositor conserva el origen de 1240×740 al redimensionar.
    x, y = (imagen.width-1240*2)//2, (imagen.height-740*2)//2
    destino = Path(__file__).resolve().parents[1] / 'replica' / 'clone-screens' / f'{nombre}.png'
    imagen.crop((x,y,x+ancho*2,y+alto*2)).save(destino)
    print(nombre, flush=True)
