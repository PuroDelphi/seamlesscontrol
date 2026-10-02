# Política de seguridad

[English](SECURITY.md)

SeamlessControl controla la entrada entre equipos. Informa de las vulnerabilidades en privado para que las personas usuarias puedan actualizar antes de divulgar detalles.

## Versiones cubiertas

Las correcciones de seguridad se dirigen a la última versión publicada y al `main` actual. No se mantienen versiones anteriores. SeamlessControl es experimental y no ha pasado una auditoría de seguridad independiente.

## Informar de una vulnerabilidad

Usa el [reporte privado de vulnerabilidades de GitHub](https://github.com/PuroDelphi/seamlesscontrol/security/advisories/new) o escribe a [soporte@asistentesautonomos.com](mailto:soporte@asistentesautonomos.com) con **SeamlessControl security** en el asunto. No abras un issue público ni compartas una prueba de explotación funcional antes de coordinar la divulgación. Incluye la versión o commit afectado, impacto, pasos para reproducir y una prueba mínima si la tienes. Elimina de los registros credenciales, secretos de emparejamiento, claves privadas y datos personales.

El mantenedor confirmará y evaluará los reportes según su disponibilidad, coordinará una corrección y divulgará el problema después de que los usuarios afectados puedan actualizar. No se promete un plazo de respuesta, recompensa ni garantía de compatibilidad. Para errores normales o dudas de instalación, consulta [SUPPORT.es.md](SUPPORT.es.md).

## Límites de seguridad

El usuario autoriza el emparejamiento en ambos equipos al comparar un código temporal. La identidad emparejada se fija localmente. El control por red usa cifrado autenticado; el receptor admite un único propietario de la entrada. El acceso del firewall debe limitarse a la interfaz LAN, subred, dirección del receptor y puerto elegidos. El panel muestra esa regla antes de pedir autorización. Estos controles no sustituyen la seguridad de los equipos ni de la red local.
