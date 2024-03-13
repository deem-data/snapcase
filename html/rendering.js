
var CY = cytoscape({
  container: document.getElementById('cy'),
  elements: [],
  style: [
    {
      selector: 'node',
      style: {
        'background-color': 'data(color)',
        'label': 'data(id)',
      }
    },
    {
      selector: 'edge',
      style: {
        'width': 3,
        'line-color': 'data(color)',
        'target-arrow-color': 'data(color)',
        'target-arrow-shape': 'triangle',
        'curve-style': 'bezier'
      }
    }
  ]
});

CY.on('tap', 'node', function(evt){
  var node = evt.target;
  const userId = parseInt(node.id());
  alert(userId);
  APP.userFocus(userId);
});

function showCards() {
    document.querySelector("#detailCards").classList.remove("d-none");
}

function renderRecommendations(recommendations) {

    var recommendedItems = "";
    var numSensitive = 0;

    for (i in recommendations) {
        const itemId = recommendations[i][0];
        const score = recommendations[i][1];

        if (APP.state.isSensitive(itemId)) {
            numSensitive += 1;
            recommendedItems += `
                <li class="list-group-item text-danger justify-content-between">

                    <span class="badge text-bg-danger rounded-pill">${score.toFixed(3)}</span>
                    <img src="images/aisles/${STATICS.aisleOfProduct[itemId]}.png" style="width: 25px; height: 25px;"/> ${STATICS.products[itemId]}

                </li>`;
        } else {
            recommendedItems += `
                <li class="list-group-item">
                    <span class="badge text-bg-light rounded-pill">${score.toFixed(3)}</span>
                    <img src="images/aisles/${STATICS.aisleOfProduct[itemId]}.png" style="width: 25px; height: 25px;"/> ${STATICS.products[itemId]}
                </li>`;
        }
    }

    document.getElementById('renderRecommendations').innerHTML = recommendedItems;
    document.getElementById('renderRecommendationStats').innerHTML = `${numSensitive} out of ${recommendations.length} items from sensitive categories`;
}

function renderEgoNetwork(egoNetwork) {
    withSensitive = egoNetwork["vertices_with_sensitive_items"];
    const cyVertices = egoNetwork["vertices"]
        .map(v => {

            var color = '#ccc';
            if (withSensitive.includes(v)) {
              color = '#ff0000';
            }

            return { data: { id: v, name: v, color: color }};
        });

    const cyEdges = egoNetwork["edges"]
        .map(e => {

            var color = '#ccc';
            if (withSensitive.includes(e[1])) {
              color = '#ff0000';
            }

            return { data: { id: e[0] + '-' + e[1], source: e[0], target: e[1], color: color }};
        });

    CY.elements().remove();
    CY.json({ elements: { nodes: cyVertices, edges: cyEdges }});

    var layout = CY.layout({
        name: 'concentric',
        animate: false,
        directed: true,
        padding: 10,
        equidistant: true,
    });
    layout.run();

    document.getElementById('renderEgoNetworkStats').innerHTML = `${withSensitive.length} out of ${cyVertices.length} users in the neighborhood bought items from sensitive categories`;
}

function renderTopAisles(topAisles) {

    var renderedTopAisles = '';
    var rank = 1;

    for (i in topAisles) {
        const aisleId = topAisles[i][0];
        const percentage = topAisles[i][1] * 100;
        const aisleName = STATICS.aisleNames[aisleId].charAt(0).toUpperCase() + STATICS.aisleNames[aisleId].slice(1);

        if (APP.state.isSensitiveAisle(aisleId)) {
            renderedTopAisles += `
                <li class="list-group-item text-danger">
                    <span class="badge text-bg-danger rounded-pill">${percentage.toFixed(2)}%</span>
                    <img src="images/aisles/${aisleId}.png" style="width: 25px; height: 25px;"/> ${rank}. ${aisleName}
                </li>
            `;
        } else {
            renderedTopAisles += `
                <li class="list-group-item">
                    <span class="badge text-bg-light rounded-pill">${percentage.toFixed(2)}%</span>
                    <img src="images/aisles/${aisleId}.png" style="width: 25px; height: 25px;"/> ${rank}. ${aisleName}
                </li>
            `;
        }
        rank++;
    }

    document.getElementById('renderTopAisles').innerHTML = renderedTopAisles;
}

function renderEmbedding(embedding) {

    var embeddingsWeights = '';

    for (i in embedding) {
        const itemId = embedding[i][0];
        const weight = embedding[i][1];

        if (APP.state.isSensitive(itemId)) {
            embeddingsWeights += `
                <li class="list-group-item text-danger">
                    ${itemId}: ${weight.toFixed(5)} (${STATICS.products[itemId]})
                </li>`;
        } else {
            embeddingsWeights += `
                <li class="list-group-item">
                    ${itemId}: ${weight.toFixed(5)} (${STATICS.products[itemId]})
                </li>`;
        }
    }
    document.getElementById('sparseUserEmbedding').innerHTML = embeddingsWeights;
    document.getElementById('sparseUserEmbeddingButton').innerHTML = `Show details (${embedding.length} items and weights)`;
}

function renderPurchases(itemPurchases) {

    var currentBasket = -1;
    var basketItems = "";

    var distinctBaskets = new Set();
    var distinctItems = new Set();

    for (i in itemPurchases) {
        var basket = itemPurchases[i]['basket_id'];
        distinctBaskets.add(basket);
        var itemId = itemPurchases[i]['item_id'];
        distinctItems.add(itemId)

        var itemName = STATICS.products[itemId];

        if (APP.state.isSensitive(itemId)) {
            basketItems += `
                <li class="list-group-item text-danger">
                <img src="images/aisles/${STATICS.aisleOfProduct[itemId]}.png" style="width: 25px; height: 25px;"/>
                ${itemName}
                <button type="button" onclick="APP.unlearnPurchase(${itemId});" class="btn btn-danger" style="--bs-btn-padding-y: .2rem; --bs-btn-padding-x: .4rem; --bs-btn-font-size: .6rem;">unlearn</button>
                </li>
            `;
        } else {
            basketItems += `
                <li class="list-group-item">
                <img src="images/aisles/${STATICS.aisleOfProduct[itemId]}.png" style="width: 25px; height: 25px;"/>
                ${itemName}
                </li>
            `;
        }

        if (basket != currentBasket) {
            const day = itemPurchases[i]["day_of_week"];
            const hour = itemPurchases[i]["hour_of_day"];

            var time = `${STATICS.weekDays[day]}, ${hour}:00`;

            if (currentBasket != -1) {
                basketItems += `</ul></p><p>${time}<ul class="list-group">`;
            } else {
                basketItems = `<p>${time}<ul class="list-group">${basketItems}`;
            }
            currentBasket = basket;
        }
    }
    basketItems += "</ul></p>";

    document.getElementById("renderPurchases").innerHTML = basketItems;
    document.getElementById("renderPurchasesStats").innerHTML = `${distinctBaskets.size} baskets with ${distinctItems.size} distinct items`;
}

function renderUnlearningChanges(deletionImpact) {

    document.getElementById('purchaseModalDetails').innerHTML = `
        Deleted ${deletionImpact['basket_ids'].length} tuples from the purchase database within ${deletionImpact['database_update_duration']}ms via the following query:
        `;

    document.getElementById('purchaseModalQuery').innerHTML = deletionImpact['deletion_query'];

    PURCHASE_MODAL.show();

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