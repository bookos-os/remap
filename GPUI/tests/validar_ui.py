"""Ejecutar con una ventana pruebas-ui y servicio_prueba.py en un bus privado."""
from control_ui import *

inicial = esperar(lambda e:e['servicio']!='cargando')
idioma = 'en' if inicial['idioma']=='Keys' else 'es'
captura(f'S01-inicio-{idioma}')
assert inicial['seleccion'] is None and not inicial['remapeos'], inicial
clic(471,133)
assert estado()['seleccion']=='esc'
captura(f'S02-editor-{idioma}')
clic(140,271)
assert esperar(lambda e:not e['guardando'])['remapeos']==[{'origen':'esc','destino':'ctrl'}]
tecla('a',codigo=30,mantener=True)
assert 30 in estado()['pulsadas']
captura(f'S06-probador-{idioma}')
enviar(accion='soltar',codigo=30)
assert not estado()['pulsadas']

# Un rechazo mantiene el estado confirmado y ofrece el motivo en la página.
(base/'rechazar').touch()
clic(140,315)
e=esperar(lambda e:e['error'] is not None)
assert e['remapeos'][0]['destino']=='ctrl' and not e['guardando'],e
captura(f'E01-error-{idioma}')
(base/'rechazar').unlink()
clic(140,315)
assert esperar(lambda e:not e['guardando'])['remapeos'][0]['destino']=='ctrl_der'
assert estado()['error'] is None

# Un servicio que dice aceptar pero no lo refleja nunca produce un éxito falso.
(base/'no_confirmar').touch()
clic(140,359)
e=esperar(lambda e:e['error'] is not None)
assert e['remapeos'][0]['destino']=='ctrl_der',e
(base/'no_confirmar').unlink()
clic(140,359)
assert esperar(lambda e:not e['guardando'])['remapeos'][0]['destino']=='shift'

# La captura global usa señales D-Bus; el teclado local recibe entrada remapeada.
clic(1130,59)
assert estado()['dialogo']=='captura'
captura(f'D01-captura-{idioma}')
(base/'capturar.json').write_text('{"codigo":240}')
assert esperar(lambda e:e['seleccion']=='evdev:240')
clic(1130,59)
(base/'capturar.json').write_text('{"codigo":193,"copilot":true}')
assert esperar(lambda e:e['seleccion']=='copilot')

# Tab queda dentro de la captura, y Enter cancela sin accionar el fondo.
clic(1130,59)
tecla('tab');tecla('enter')
assert esperar(lambda e:e['dialogo']=='ninguno')
assert (base/'armada').read_text()=='False'

# Un tramo vertical no puede transformarse en atrás por una diagonal tardía.
enviar(accion='rueda',x=700,y=600,dx=2,dy=10)
enviar(accion='rueda',x=700,y=600,dx=200,dy=0)
time.sleep(.8)
assert estado()['seleccion']=='copilot'
# Un tramo nuevo que supera media anchura sí vuelve al inicio.
enviar(accion='rueda',x=700,y=600,dx=200,dy=0)
assert esperar(lambda e:e['seleccion'] is None)
captura(f'S01-remapeos-{idioma}')

# El tamaño mínimo conserva las seis filas del teclado.
enviar(accion='tamano',ancho=1080,alto=620)
time.sleep(.3)
assert estado()['tamano']==[1080,620]
captura(f'S01-minimo-{idioma}')
enviar(accion='tamano',ancho=1240,alto=740)
print(json.dumps({'idioma':idioma,'motor':inicial['servicio'],'resultado':'correcto'},ensure_ascii=False))
