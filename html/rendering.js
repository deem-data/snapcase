
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
  alert(node.id());
  APP.userFocus(node.id());
});

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

