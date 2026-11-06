// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.network.models

data class TemplateResult(
    val hash: String = "",
    val status: String = "",
    val success: Boolean = false,
    val errorMessage: String? = null
)
