package com.kanari.kanari_crypto.compose

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.kanari.kanari_crypto.model.CurveInfoModel

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun CurveSelector(
    curves: List<CurveInfoModel>,
    selectedCurve: String,
    onCurveSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    FlowRow(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        curves.forEach { curve ->
            FilterChip(
                selected = curve.name == selectedCurve,
                onClick = { onCurveSelected(curve.name) },
                label = { Text(curve.name) },
                leadingIcon = if (curve.isPostQuantum) {
                    {
                        Text(
                            text = "PQ",
                            style = MaterialTheme.typography.labelSmall,
                        )
                    }
                } else {
                    null
                },
            )
        }
    }
}
