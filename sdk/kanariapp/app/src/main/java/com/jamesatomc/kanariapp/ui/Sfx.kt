// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.ui

import android.content.Context
import android.media.AudioAttributes
import android.media.SoundPool
import androidx.core.content.edit
import com.jamesatomc.kanariapp.R

/**
 * Tiny SoundPool-backed UI sound kit (tap blip + send-success chime).
 *
 * Initialized lazily on first use and kept for the process lifetime — the
 * two clips are a few KB each, so there is nothing worth releasing between
 * screens. All calls are no-ops if loading ever fails, so audio can never
 * break a transfer. Users can turn the whole kit off from Settings; the
 * choice persists in the app's shared preferences.
 */
object Sfx {
    private const val PREFS = "kanari_prefs"
    private const val KEY_ENABLED = "sound_effects_enabled"

    @Volatile
    private var pool: SoundPool? = null

    @Volatile
    private var tapId = 0

    @Volatile
    private var successId = 0

    @Volatile
    private var cachedEnabled: Boolean? = null

    fun isEnabled(context: Context): Boolean =
        cachedEnabled ?: context.applicationContext
            .getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getBoolean(KEY_ENABLED, true)
            .also { cachedEnabled = it }

    fun setEnabled(context: Context, value: Boolean) {
        cachedEnabled = value
        context.applicationContext
            .getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit { putBoolean(KEY_ENABLED, value) }
    }

    private fun ensure(context: Context) {
        if (pool != null) return
        synchronized(this) {
            if (pool != null) return
            val app = context.applicationContext
            val p = SoundPool.Builder()
                .setMaxStreams(3)
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_ASSISTANCE_SONIFICATION)
                        .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                        .build(),
                )
                .build()
            runCatching {
                tapId = p.load(app, R.raw.tap, 1)
                successId = p.load(app, R.raw.success, 1)
            }
            pool = p
        }
    }

    fun tap(context: Context) {
        if (!isEnabled(context)) return
        ensure(context)
        runCatching { pool?.play(tapId, 0.5f, 0.5f, 1, 0, 1f) }
    }

    fun success(context: Context) {
        if (!isEnabled(context)) return
        ensure(context)
        runCatching { pool?.play(successId, 0.9f, 0.9f, 1, 0, 1f) }
    }
}
