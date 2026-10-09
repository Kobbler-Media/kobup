package com.kobup.app.ui.components

import androidx.compose.foundation.border
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
fun Modifier.tvFocusable(
    focusedBorderColor: Color = MaterialTheme.colorScheme.primary,
    borderWidth: Dp = 3.dp,
    shape: RoundedCornerShape = RoundedCornerShape(12.dp),
): Modifier {
    var isFocused by remember { mutableStateOf(false) }

    return this
        .onFocusChanged { isFocused = it.isFocused }
        .then(
            if (isFocused) {
                Modifier.border(borderWidth, focusedBorderColor, shape)
            } else {
                Modifier
            }
        )
}

fun FocusRequester.requestFocusSafely() {
    try {
        requestFocus()
    } catch (_: IllegalStateException) {
    }
}
