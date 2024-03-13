
class AppState {
    constructor(sensitiveCategories) {
        this.sensitiveCategories = sensitiveCategories;
        this.currentUser = undefined;
    }

    isSensitive(itemId) {
        const category = STATICS.aisleOfProduct[itemId];
        return this.sensitiveCategories.includes(category);
    }

    isSensitiveAisle(aisleId) {
        return this.sensitiveCategories.includes(aisleId);
    }
}

class App {

    constructor() {

        this.state = new AppState([27, 28, 62, 124, 134]);
        this.socket = new WebSocket("ws://localhost:8080/ws");

        this.socket.onmessage = function (event) {
            const response = JSON.parse(event.data);
            if (response["response_type"] == "purchases") {
                renderPurchases(response["payload"]["item_purchases"]);
            } else if (response["response_type"] == "model_state") {
                renderEmbedding(response["payload"]["embedding"]["weights_per_item"]);
                renderEgoNetwork(response["payload"]["ego_network"]);
                renderTopAisles(response["payload"]["ego_network"]["top_aisles"]);
            } else if (response["response_type"] == "recommendations") {
                renderRecommendations(response["payload"]);
            } else if (response["response_type"] == "deletion_impact") {
                APP.startUnlearningJourney(response["payload"]);
            }
        }
    }

    userFocus(userId) {
        this.state.currentUserId = userId;
        this.requestPurchases(userId);
        this.requestModelState(userId);
        this.requestRecommendations(userId);
    }

    startUnlearningJourney(deletionImpact) {

        document.getElementById('purchaseModalDetails').innerHTML = `
            Deleted ${deletionImpact['basket_ids'].length} tuples from the purchase database within ${deletionImpact['database_update_duration']}ms via the following query:
            `;

        document.getElementById('purchaseModalQuery').innerHTML = deletionImpact['deletion_query'];

        const purchaseModal = new bootstrap.Modal('#purchaseModal', {});
        purchaseModal.show();

        document.getElementById('modelModalEmbeddingStats').innerHTML = `Update of the user embedding took ${deletionImpact['embedding_update_duration']} ms`;

        var embeddingDifferences = '';
        for (var i in deletionImpact['embedding_difference']) {
            const itemId = deletionImpact['embedding_difference'][i][0];
            const change = deletionImpact['embedding_difference'][i][1];

            if (APP.state.isSensitive(itemId)) {
                embeddingDifferences += `
                    <li class="list-group-item text-danger">
                    ${itemId}: ${change} (${STATICS.products[itemId]})
                    </li>
                `;
            } else {
                embeddingDifferences += `
                    <li class="list-group-item">
                    ${itemId}: ${change} (${STATICS.products[itemId]})
                    </li>
                `;
            }
        }
        document.getElementById('modelModalEmbeddingChanges').innerHTML = embeddingDifferences;

        var num_deleted_adjacent = 0;
        var num_updated_adjacent = 0;
        var num_inserted_adjacent = 0;
        for (var i in deletionImpact['adjacent_difference']) {
            const change = deletionImpact['adjacent_difference'][i][2];
            if (change == "Delete") {
                num_deleted_adjacent++;
            } else if (change == "Update") {
                num_updated_adjacent++;
            } else {
                num_inserted_adjacent++;
            }
        }

        var num_deleted_incident = 0;
        var num_updated_incident = 0;
        var num_inserted_incident = 0;
        for (var i in deletionImpact['incident_difference']) {
            const change = deletionImpact['incident_difference'][i][2];
            if (change == "Delete") {
                num_deleted_incident++;
            } else if (change == "Update") {
                num_updated_incident++;
            } else {
                num_inserted_incident++;
            }
        }

        var indexUpdateStats = `
            <li>Update of the top-k index took ${deletionImpact['topk_index_update_duration']} ms</li>
            <li>Maintenance of the top-k index involved inspection of the entries for ${deletionImpact['num_inspected_neighbors']} users and updates for the entries of ${deletionImpact['num_updated_neighbors']} users</li>
            <li>Changes in the neighbors determining the recommendations for this user: ${num_deleted_adjacent} deletions, ${num_inserted_adjacent} insertions, ${num_updated_adjacent} weight updates</li>
            <li>Changes in the neighbors whose recommendations are determined by this user: ${num_deleted_incident} deletions, ${num_inserted_incident} insertions, ${num_updated_incident} weight updates</li>
        `;

        document.getElementById('modelModelIndexUpdateStats').innerHTML = indexUpdateStats;


        const arrowUp = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-up" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M8 15a.5.5 0 0 0 .5-.5V2.707l3.146 3.147a.5.5 0 0 0 .708-.708l-4-4a.5.5 0 0 0-.708 0l-4 4a.5.5 0 1 0 .708.708L7.5 2.707V14.5a.5.5 0 0 0 .5.5"/>
            </svg>
        `;

        const arrowDown = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-down" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M8 1a.5.5 0 0 1 .5.5v11.793l3.146-3.147a.5.5 0 0 1 .708.708l-4 4a.5.5 0 0 1-.708 0l-4-4a.5.5 0 0 1 .708-.708L7.5 13.293V1.5A.5.5 0 0 1 8 1"/>
            </svg>
        `;

        var recommendationChanges = {
            'Delete': "",
            'Insert': "",
            'Update': "",
        };

        for (var i in deletionImpact['recommendation_difference']) {
            const itemId = deletionImpact['recommendation_difference'][i][0];
            const diff = deletionImpact['recommendation_difference'][i][1];
            const change = deletionImpact['recommendation_difference'][i][2];

            const arrow = diff > 0 ? arrowUp : arrowDown;
            const renderedDiff = diff > 0 ? '+' + diff.toFixed(4) : diff.toFixed(4);

            if (APP.state.isSensitive(itemId)) {
                recommendationChanges[change] += `
                    <li class="list-group-item text-danger">
                        <img src="images/aisles/${STATICS.aisleOfProduct[itemId]}.png" style="width: 25px; height: 25px;"/> ${STATICS.products[itemId]}
                        ${arrow} ${renderedDiff}
                    </li>`;
            } else {
                recommendationChanges[change] += `
                    <li class="list-group-item">
                        <img src="images/aisles/${STATICS.aisleOfProduct[itemId]}.png" style="width: 25px; height: 25px;"/> ${STATICS.products[itemId]}
                        ${arrow} ${renderedDiff}
                    </li>`;
            }

        }

        document.getElementById('recommendationChangesDeletes').innerHTML = recommendationChanges['Delete'];
        document.getElementById('recommendationChangesInserts').innerHTML = recommendationChanges['Insert'];
        document.getElementById('recommendationChangesUpdates').innerHTML = recommendationChanges['Update'];

        var influenceChanges = {
            'Delete': "",
            'Insert': "",
            'Update': "",
        };

        for (var i in deletionImpact['top_aisle_difference']) {
            const aisleId = deletionImpact['top_aisle_difference'][i][0];
            const diff = deletionImpact['top_aisle_difference'][i][1];
            const change = deletionImpact['top_aisle_difference'][i][2];

            const arrow = diff > 0 ? arrowUp : arrowDown;
            const renderedDiff = diff > 0 ? '+' + diff.toFixed(4) : diff.toFixed(4);

            const aisleName = STATICS.aisleNames[aisleId].charAt(0).toUpperCase() + STATICS.aisleNames[aisleId].slice(1);

            if (APP.state.isSensitiveAisle(aisleId)) {
                influenceChanges[change] += `
                    <li class="list-group-item text-danger">
                        <img src="images/aisles/${aisleId}.png" style="width: 25px; height: 25px;"/> ${aisleName}
                        ${arrow} ${renderedDiff}
                    </li>`;
            } else {
                influenceChanges[change] += `
                    <li class="list-group-item">
                        <img src="images/aisles/${aisleId}.png" style="width: 25px; height: 25px;"/> ${aisleName}
                        ${arrow} ${renderedDiff}
                    </li>`;
            }
        }

        document.getElementById('influenceChangesDeletes').innerHTML = influenceChanges['Delete'];
        document.getElementById('influenceChangesInserts').innerHTML = influenceChanges['Insert'];
        document.getElementById('influenceChangesUpdates').innerHTML = influenceChanges['Update'];
    }

    requestPurchases(userId) {
        this.socket.send(JSON.stringify({ "Purchases": { "user_id": userId } }));
    }

    requestModelState(userId) {
        this.socket.send(JSON.stringify({ "ModelState": { "user_id": userId } }));
    }

    requestRecommendations(userId) {
        this.socket.send(JSON.stringify({ "Recommendations": { "user_id": userId } }));
    }

    unlearnPurchase(itemId) {
        this.socket.send(JSON.stringify({ "PurchaseDeletion": { "user_id": this.state.currentUserId, "item_id": itemId } }));
    }
}
