package com.kobup.app

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity

/**
 * Handles com.kobup.action.OPEN_APP intents from the Kobup SDK.
 * Opens the Kobup catalog, optionally navigating to a specific app.
 */
class OpenAppActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val mainIntent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            // Forward any extras (e.g., package_name) so MainActivity can deep-link
            intent.extras?.let { putExtras(it) }
        }
        startActivity(mainIntent)
        finish()
    }
}
