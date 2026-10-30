# Launcher artwork

Unmodified official NetEase game artwork. Rights remain with NetEase.

- Background (1920x1080): https://loadingbaycn.fp.ps.netease.com/file/67b43dd5b2e232e0dd3848cdEGSADZyK06
- Color logo (400x200): https://loadingbaycn.fp.ps.netease.com/file/67ce582270fb9f4964153705M0VUmt3106
- Both confirmed on 2026-10-30 from https://loadingbaycn.webapp.163.com/app/v1/game_library/app?force=1&app_id=73 (channel: mkt-h55).
- Current idv-login CloudRes.get_login_page prioritizes login_base64_page_fever. That UI uses background_image and game_library_logo; the legacy fallback page uses main_image (only 480x270), which is unsuitable for the full background.
- Source: https://github.com/KKeygen/idv-login/blob/main/src/cloudRes.py and https://github.com/KKeygen/idv-login/blob/main/assets/cloudRes.json.
